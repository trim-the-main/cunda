import 'dart:convert';
import 'dart:io';

import 'package:logging/logging.dart';

import 'known_device_tracker.dart';
import 'mayna_types.dart';

final _log = Logger('KnownDeviceTracker');

class KnownDeviceTrackerImpl extends KnownDeviceTracker {
  final File _storageFile;
  final List<KnownDevice> _devices = [];

  KnownDeviceTrackerImpl._(this._storageFile);

  static Future<KnownDeviceTrackerImpl> init(File storageFile) async {
    final tracker = KnownDeviceTrackerImpl._(storageFile);
    await tracker._load();
    return tracker;
  }

  Future<void> _load() async {
    _devices.clear();
    if (await _storageFile.exists()) {
      final json = jsonDecode(await _storageFile.readAsString());
      final list = json['devices'] as List<dynamic>;
      for (final item in list) {
        _devices.add(KnownDevice.fromJson(item as Map<String, dynamic>));
      }
      _log.fine('Loaded ${_devices.length} known devices');
    }
  }

  Future<void> _save() async {
    final json = {
      'devices': _devices.map((d) => d.toJson()).toList(),
    };
    await _storageFile.create(recursive: true);
    await _storageFile.writeAsString(jsonEncode(json));
  }

  @override
  Future<void> onDeviceConnected(
      String deviceType, int serialNumber, String firmwareVersion) async {
    final index =
        _devices.indexWhere((d) => d.matches(deviceType, serialNumber));
    final device = KnownDevice(
      serialNumber: serialNumber,
      deviceType: deviceType,
      firmwareVersion: firmwareVersion,
      lastSeen: DateTime.now(),
    );
    if (index >= 0) {
      _devices[index] = device;
      _log.fine('Updated known device $deviceType/$serialNumber');
    } else {
      _devices.add(device);
      _log.info(
          'New known device: $deviceType/$serialNumber (fw $firmwareVersion)');
    }
    await _save();
  }

  @override
  Future<void> forget(String deviceType, int serialNumber) async {
    _devices.removeWhere((d) => d.matches(deviceType, serialNumber));
    await _save();
    _log.info('Forgot device $deviceType/$serialNumber');
  }

  @override
  KnownDevice? find(String deviceType, int serialNumber) {
    for (final d in _devices) {
      if (d.matches(deviceType, serialNumber)) return d;
    }
    return null;
  }

  @override
  List<KnownDevice> listAll() => List.unmodifiable(_devices);

  @override
  List<KnownDevice> listByDeviceType(String deviceType) =>
      _devices.where((d) => d.deviceType == deviceType).toList();

  @override
  Set<String> activeVersions(String deviceType) => _devices
      .where((d) => d.deviceType == deviceType)
      .map((d) => d.firmwareVersion)
      .toSet();

  @override
  Set<String> knownDeviceTypes() =>
      _devices.map((d) => d.deviceType).toSet();
}
