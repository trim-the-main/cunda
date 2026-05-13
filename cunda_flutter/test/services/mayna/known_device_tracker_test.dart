import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

import 'package:cunda_flutter/services/mayna/known_device_tracker.dart';
import 'package:cunda_flutter/services/mayna/mayna_types.dart';

void main() {
  late Directory tempDir;
  late KnownDeviceTracker tracker;

  setUp(() async {
    tempDir = await Directory.systemTemp.createTemp('mayna_test_');
    final storageFile = File('${tempDir.path}/known_devices.json');
    tracker = await KnownDeviceTracker.init(storageFile);
  });

  tearDown(() async {
    await tempDir.delete(recursive: true);
  });

  group('KnownDeviceTracker', () {
    test('listAll returns empty initially', () {
      expect(tracker.listAll(), isEmpty);
    });

    test('onDeviceConnected adds a new device', () async {
      await tracker.onDeviceConnected('cunda', 1001, '1.0.0');

      final devices = tracker.listAll();
      expect(devices, hasLength(1));
      expect(devices.first.serialNumber, 1001);
      expect(devices.first.deviceType, 'cunda');
      expect(devices.first.firmwareVersion, '1.0.0');
    });

    test('onDeviceConnected updates existing device by identity', () async {
      await tracker.onDeviceConnected('cunda', 1001, '1.0.0');
      final firstSeen = tracker.listAll().first.lastSeen;

      await Future.delayed(const Duration(milliseconds: 10));
      await tracker.onDeviceConnected('cunda', 1001, '1.0.0');

      final devices = tracker.listAll();
      expect(devices, hasLength(1));
      expect(devices.first.lastSeen.isAfter(firstSeen), isTrue);
    });

    test('onDeviceConnected updates firmware version', () async {
      await tracker.onDeviceConnected('cunda', 1001, '1.0.0');
      await tracker.onDeviceConnected('cunda', 1001, '2.0.0');

      final devices = tracker.listAll();
      expect(devices, hasLength(1));
      expect(devices.first.firmwareVersion, '2.0.0');
    });

    test('multiple devices of same type tracked independently', () async {
      await tracker.onDeviceConnected('cunda', 1001, '1.0.0');
      await tracker.onDeviceConnected('cunda', 1002, '2.0.0');

      expect(tracker.listAll(), hasLength(2));
      expect(tracker.listByDeviceType('cunda'), hasLength(2));
      expect(tracker.activeVersions('cunda'), {'1.0.0', '2.0.0'});
    });

    test('same serial across different device types are distinct', () async {
      await tracker.onDeviceConnected('cunda', 1001, '1.0.0');
      await tracker.onDeviceConnected('other', 1001, '3.0.0');

      expect(tracker.listAll(), hasLength(2));
      expect(tracker.find('cunda', 1001)!.firmwareVersion, '1.0.0');
      expect(tracker.find('other', 1001)!.firmwareVersion, '3.0.0');
    });

    test('forget removes a device by identity', () async {
      await tracker.onDeviceConnected('cunda', 1001, '1.0.0');
      await tracker.onDeviceConnected('cunda', 1002, '1.0.0');
      await tracker.forget('cunda', 1001);

      expect(tracker.listAll(), hasLength(1));
      expect(tracker.listAll().first.serialNumber, 1002);
    });

    test('forget with same serial different type only removes target', () async {
      await tracker.onDeviceConnected('cunda', 1001, '1.0.0');
      await tracker.onDeviceConnected('other', 1001, '1.0.0');
      await tracker.forget('cunda', 1001);

      expect(tracker.listAll(), hasLength(1));
      expect(tracker.listAll().first.deviceType, 'other');
    });

    test('find returns correct device', () async {
      await tracker.onDeviceConnected('cunda', 1001, '1.0.0');
      await tracker.onDeviceConnected('other', 1002, '2.0.0');

      expect(tracker.find('cunda', 1001)!.firmwareVersion, '1.0.0');
      expect(tracker.find('cunda', 9999), isNull);
    });

    test('listByDeviceType filters correctly', () async {
      await tracker.onDeviceConnected('cunda', 1001, '1.0.0');
      await tracker.onDeviceConnected('other', 1002, '1.0.0');

      expect(tracker.listByDeviceType('cunda'), hasLength(1));
      expect(tracker.listByDeviceType('unknown'), isEmpty);
    });

    test('activeVersions returns firmware versions for device type', () async {
      await tracker.onDeviceConnected('cunda', 1001, '1.0.0');
      await tracker.onDeviceConnected('cunda', 1002, '2.0.0');

      expect(tracker.activeVersions('cunda'), {'1.0.0', '2.0.0'});
      expect(tracker.activeVersions('other'), isEmpty);
    });

    test('knownDeviceTypes returns all types', () async {
      await tracker.onDeviceConnected('cunda', 1001, '1.0.0');
      await tracker.onDeviceConnected('other', 1002, '2.0.0');

      expect(tracker.knownDeviceTypes(), {'cunda', 'other'});
    });

    test('persists across instances', () async {
      final storageFile = File('${tempDir.path}/persist_test.json');
      final tracker1 = await KnownDeviceTracker.init(storageFile);
      await tracker1.onDeviceConnected('cunda', 1001, '1.0.0');

      final tracker2 = await KnownDeviceTracker.init(storageFile);
      expect(tracker2.listAll(), hasLength(1));
      expect(tracker2.listAll().first.serialNumber, 1001);
      expect(tracker2.listAll().first.deviceType, 'cunda');
    });
  });
}
