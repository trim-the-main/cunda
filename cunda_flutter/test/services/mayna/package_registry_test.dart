import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

import 'package:cunda_flutter/services/mayna/mayna_types.dart';
import 'package:cunda_flutter/services/mayna/package_registry.dart';

import 'test_helpers.dart';

void main() {
  late Directory tempDir;
  late PackageRegistry registry;

  setUp(() async {
    tempDir = await Directory.systemTemp.createTemp('mayna_test_');
    registry = await PackageRegistry.init(tempDir);
  });

  tearDown(() async {
    await tempDir.delete(recursive: true);
  });

  group('PackageRegistry', () {
    test('listAll returns empty initially', () {
      expect(registry.listAll(), isEmpty);
    });

    test('import adds a package', () async {
      final maynaFile = await createTestMaynaFile(
        tempDir,
        deviceType: 'cunda',
        firmwareVersion: '1.0.0',
        protocolVersion: 1,
      );

      final manifest = await registry.import(maynaFile);
      expect(manifest.deviceType, 'cunda');
      expect(manifest.firmwareVersion, '1.0.0');
      expect(manifest.protocolVersion, 1);
      expect(registry.listAll(), hasLength(1));
    });

    test('find returns the correct package', () async {
      await importTestPackage(registry, tempDir,
          deviceType: 'cunda', firmwareVersion: '1.0.0');

      expect(registry.find('cunda', '1.0.0'), isNotNull);
      expect(registry.find('cunda', '2.0.0'), isNull);
      expect(registry.find('other', '1.0.0'), isNull);
    });

    test('listByDeviceType filters correctly', () async {
      await importTestPackage(registry, tempDir,
          deviceType: 'cunda', firmwareVersion: '1.0.0');
      await importTestPackage(registry, tempDir,
          deviceType: 'cunda', firmwareVersion: '2.0.0');
      await importTestPackage(registry, tempDir,
          deviceType: 'other', firmwareVersion: '1.0.0');

      expect(registry.listByDeviceType('cunda'), hasLength(2));
      expect(registry.listByDeviceType('other'), hasLength(1));
      expect(registry.listByDeviceType('unknown'), isEmpty);
    });

    test('remove deletes a package', () async {
      await importTestPackage(registry, tempDir,
          deviceType: 'cunda', firmwareVersion: '1.0.0');
      expect(registry.listAll(), hasLength(1));

      await registry.remove('cunda', '1.0.0');
      expect(registry.listAll(), isEmpty);
      expect(registry.find('cunda', '1.0.0'), isNull);
    });

    test('componentPath returns correct path for installed package', () async {
      await importTestPackage(registry, tempDir,
          deviceType: 'cunda', firmwareVersion: '1.0.0');

      final path = registry.componentPath('cunda', '1.0.0', 'firmware');
      expect(path, isNotNull);
      expect(path!.existsSync(), isTrue);
    });

    test('componentPath returns null for missing package', () {
      expect(registry.componentPath('cunda', '1.0.0', 'firmware'), isNull);
    });

    test('componentPath returns null for missing component', () async {
      await importTestPackage(registry, tempDir,
          deviceType: 'cunda', firmwareVersion: '1.0.0');

      expect(
          registry.componentPath('cunda', '1.0.0', 'nonexistent'), isNull);
    });

    test('findLatestCompatible respects protocol version', () async {
      await importTestPackage(registry, tempDir,
          deviceType: 'cunda',
          firmwareVersion: '1.0.0',
          protocolVersion: 1);
      await importTestPackage(registry, tempDir,
          deviceType: 'cunda',
          firmwareVersion: '2.0.0',
          protocolVersion: 2);

      final result = registry.findLatestCompatible(
        deviceType: 'cunda',
        currentFirmwareVersion: '1.0.0',
        supportedProtocols: {1},
      );
      expect(result, isNotNull);
      expect(result!.firmwareVersion, '1.0.0');
    });

    test('findLatestCompatible respects minFirmwareVersion', () async {
      await importTestPackage(registry, tempDir,
          deviceType: 'cunda',
          firmwareVersion: '1.0.0',
          protocolVersion: 1,
          minFirmwareVersion: '0.0.0');
      await importTestPackage(registry, tempDir,
          deviceType: 'cunda',
          firmwareVersion: '2.0.0',
          protocolVersion: 1,
          minFirmwareVersion: '1.5.0');

      // Current version 1.0.0 < minFirmwareVersion 1.5.0, so 2.0.0 is excluded
      final result = registry.findLatestCompatible(
        deviceType: 'cunda',
        currentFirmwareVersion: '1.0.0',
        supportedProtocols: {1},
      );
      expect(result, isNotNull);
      expect(result!.firmwareVersion, '1.0.0');
    });

    test('findLatestCompatible returns null when nothing matches', () async {
      await importTestPackage(registry, tempDir,
          deviceType: 'cunda',
          firmwareVersion: '1.0.0',
          protocolVersion: 2);

      final result = registry.findLatestCompatible(
        deviceType: 'cunda',
        currentFirmwareVersion: '0.5.0',
        supportedProtocols: {1},
      );
      expect(result, isNull);
    });

    test('findCleanablePackages identifies removable packages', () async {
      await importTestPackage(registry, tempDir,
          deviceType: 'cunda',
          firmwareVersion: '1.0.0',
          protocolVersion: 1);
      await importTestPackage(registry, tempDir,
          deviceType: 'cunda',
          firmwareVersion: '2.0.0',
          protocolVersion: 1);
      await importTestPackage(registry, tempDir,
          deviceType: 'cunda',
          firmwareVersion: '3.0.0',
          protocolVersion: 1);

      // Device running 2.0.0; latest compatible is 3.0.0; 1.0.0 is cleanable
      final knownDevices = [
        KnownDevice(
          serialNumber: 1001,
          deviceType: 'cunda',
          firmwareVersion: '2.0.0',
          lastSeen: DateTime.now(),
        ),
      ];

      final cleanable = registry.findCleanablePackages(knownDevices);
      expect(cleanable, hasLength(1));
      expect(cleanable.first.firmwareVersion, '1.0.0');
    });

    test('cleanup removes cleanable packages', () async {
      await importTestPackage(registry, tempDir,
          deviceType: 'cunda',
          firmwareVersion: '1.0.0',
          protocolVersion: 1);
      await importTestPackage(registry, tempDir,
          deviceType: 'cunda',
          firmwareVersion: '2.0.0',
          protocolVersion: 1);

      final knownDevices = <KnownDevice>[];
      // No known devices — only the latest per device type is retained
      final removed = await registry.cleanup(knownDevices);
      expect(removed, 1);
      expect(registry.listAll(), hasLength(1));
      expect(registry.find('cunda', '2.0.0'), isNotNull);
    });

    test('reconciliation cleans up tmp directory on init', () async {
      // Create a stale .tmp directory
      final tmpDir = Directory('${tempDir.path}/packages/.tmp');
      await tmpDir.create(recursive: true);
      await File('${tmpDir.path}/junk.bin').writeAsBytes([1, 2, 3]);

      // Re-init should clean it up
      final freshRegistry = await PackageRegistry.init(tempDir);
      expect(tmpDir.existsSync(), isFalse);
      expect(freshRegistry.listAll(), isEmpty);
    });

    test('import overwrites existing package of same version', () async {
      await importTestPackage(registry, tempDir,
          deviceType: 'cunda', firmwareVersion: '1.0.0');
      await importTestPackage(registry, tempDir,
          deviceType: 'cunda', firmwareVersion: '1.0.0');

      expect(registry.listAll(), hasLength(1));
    });
  });
}
