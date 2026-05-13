import 'dart:io';

import 'known_device_tracker_impl.dart';
import 'mayna_types.dart';

/// Tracks which devices the app has connected to.
///
/// Entries are created implicitly on first successful BLE connection
/// and updated when firmware versions change (e.g. after OTA).
abstract class KnownDeviceTracker {
  /// Initialize from known_devices.json.
  static Future<KnownDeviceTracker> init(File storageFile) =>
      KnownDeviceTrackerImpl.init(storageFile);

  /// Record a device connection. Creates or updates the entry.
  Future<void> onDeviceConnected(
      String deviceType, int serialNumber, String firmwareVersion);

  /// Explicitly remove a device from known devices.
  Future<void> forget(String deviceType, int serialNumber);

  /// Find a known device by its (deviceType, serialNumber) identity.
  KnownDevice? find(String deviceType, int serialNumber);

  /// List all known devices.
  List<KnownDevice> listAll();

  /// List known devices of a given type.
  List<KnownDevice> listByDeviceType(String deviceType);

  /// Get the set of firmware versions in use by known devices
  /// of a given type.
  Set<String> activeVersions(String deviceType);

  /// Get all device types that are known.
  Set<String> knownDeviceTypes();
}
