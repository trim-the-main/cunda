import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

import 'package:cunda_flutter/services/mayna/mayna_types.dart';
import 'package:cunda_flutter/services/mayna/package_registry.dart';
import 'package:cunda_flutter/services/mayna/remote_package_source.dart';

import 'fake_remote_package_source.dart';
import 'test_helpers.dart';

void main() {
  late Directory tempDir;

  setUp(() async {
    tempDir = await Directory.systemTemp.createTemp('mayna_test_');
  });

  tearDown(() async {
    await tempDir.delete(recursive: true);
  });

  group('RemotePackageSource', () {
    test('cached is null before refresh', () {
      final source = FakeRemotePackageSource(remotePackages: []);
      expect(source.cached, isNull);
    });

    test('refresh returns manifest with sequence number', () async {
      final source = FakeRemotePackageSource(
          remotePackages: [], initialSequence: 42);

      final manifest = await source.refresh();
      expect(manifest.sequenceNumber, 42);

      final manifest2 = await source.refresh();
      expect(manifest2.sequenceNumber, 43);
    });

    test('refresh returns the remote manifest', () async {
      final source = FakeRemotePackageSource(remotePackages: [
        RemotePackageInfo(
          deviceType: 'cunda',
          firmwareVersion: '1.0.0',
          protocolVersion: 1,
          minFirmwareVersion: '0.0.0',
          publishDate: '2026-01-01',
          packageUrl: Uri.parse('https://example.com/cunda_1.0.0.mayna'),
          packageSize: 1024,
          packageSha256: 'abc123',
        ),
      ]);

      final manifest = await source.refresh();
      expect(manifest.packages, hasLength(1));
      expect(source.cached, isNotNull);
    });

    test('findAvailable excludes locally installed packages', () async {
      final registry = await PackageRegistry.init(tempDir);
      await importTestPackage(registry, tempDir,
          deviceType: 'cunda', firmwareVersion: '1.0.0');

      final source = FakeRemotePackageSource(remotePackages: [
        RemotePackageInfo(
          deviceType: 'cunda',
          firmwareVersion: '1.0.0',
          protocolVersion: 1,
          minFirmwareVersion: '0.0.0',
          publishDate: '2026-01-01',
          packageUrl: Uri.parse('https://example.com/cunda_1.0.0.mayna'),
          packageSize: 1024,
          packageSha256: 'abc123',
        ),
        RemotePackageInfo(
          deviceType: 'cunda',
          firmwareVersion: '2.0.0',
          protocolVersion: 1,
          minFirmwareVersion: '0.0.0',
          publishDate: '2026-02-01',
          packageUrl: Uri.parse('https://example.com/cunda_2.0.0.mayna'),
          packageSize: 2048,
          packageSha256: 'def456',
        ),
      ]);
      await source.refresh();

      final available = source.findAvailable(
        deviceType: 'cunda',
        localRegistry: registry,
      );
      expect(available, hasLength(1));
      expect(available.first.firmwareVersion, '2.0.0');
    });

    test('findAvailable filters by device type', () async {
      final registry = await PackageRegistry.init(tempDir);

      final source = FakeRemotePackageSource(remotePackages: [
        RemotePackageInfo(
          deviceType: 'cunda',
          firmwareVersion: '1.0.0',
          protocolVersion: 1,
          minFirmwareVersion: '0.0.0',
          publishDate: '2026-01-01',
          packageUrl: Uri.parse('https://example.com/cunda_1.0.0.mayna'),
          packageSize: 1024,
          packageSha256: 'abc123',
        ),
        RemotePackageInfo(
          deviceType: 'other',
          firmwareVersion: '1.0.0',
          protocolVersion: 1,
          minFirmwareVersion: '0.0.0',
          publishDate: '2026-01-01',
          packageUrl: Uri.parse('https://example.com/other_1.0.0.mayna'),
          packageSize: 1024,
          packageSha256: 'ghi789',
        ),
      ]);
      await source.refresh();

      final available = source.findAvailable(
        deviceType: 'cunda',
        localRegistry: registry,
      );
      expect(available, hasLength(1));
      expect(available.first.deviceType, 'cunda');
    });

    test('download returns a file', () async {
      final source = FakeRemotePackageSource(remotePackages: []);
      final info = RemotePackageInfo(
        deviceType: 'cunda',
        firmwareVersion: '1.0.0',
        protocolVersion: 1,
        minFirmwareVersion: '0.0.0',
        publishDate: '2026-01-01',
        packageUrl: Uri.parse('https://example.com/cunda_1.0.0.mayna'),
        packageSize: 1024,
        packageSha256: 'abc123',
      );

      final file = await source.download(info);
      expect(file.existsSync(), isTrue);
    });
  });
}
