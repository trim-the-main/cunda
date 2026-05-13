import 'dart:io';

import 'mayna_types.dart';
import 'package_registry_impl.dart';

/// Manages locally installed .mayna firmware packages.
///
/// Handles import, query, removal, and cleanup of packages stored
/// on the filesystem. Uses atomic rename for crash-safe extraction.
abstract class PackageRegistry {
  /// Initialize the registry at the given base directory.
  ///
  /// Runs startup reconciliation:
  ///   1. Delete all .tmp/ directories
  ///   2. Remove local_manifest entries for missing directories
  ///   3. Add entries for directories present but missing from local_manifest
  static Future<PackageRegistry> init(Directory baseDir) =>
      PackageRegistryImpl.init(baseDir);

  /// Import a .mayna package from a file.
  ///
  /// 1. Extract ZIP to packages/.tmp/
  /// 2. Verify each component: file presence, size, SHA256
  /// 3. Atomic rename .tmp/ → <deviceType>_<firmwareVersion>/
  /// 4. Update local_manifest.json
  ///
  /// On any verification failure: delete .tmp/, return error.
  Future<MaynaPackageManifest> import(File maynaFile);

  /// Remove a specific installed package.
  Future<void> remove(String deviceType, String firmwareVersion);

  /// List all installed packages.
  List<MaynaPackageManifest> listAll();

  /// List installed packages for a device type.
  List<MaynaPackageManifest> listByDeviceType(String deviceType);

  /// Find a specific installed package.
  MaynaPackageManifest? find(String deviceType, String firmwareVersion);

  /// Find the latest installed package compatible with a device.
  ///
  /// Filters by: protocolVersion in [supportedProtocols],
  ///             minFirmwareVersion <= [currentFirmwareVersion].
  MaynaPackageManifest? findLatestCompatible({
    required String deviceType,
    required String currentFirmwareVersion,
    required Set<int> supportedProtocols,
  });

  /// Build an update chain when direct update is blocked by
  /// minFirmwareVersion constraints.
  ///
  /// Returns null if no path exists.
  List<MaynaPackageManifest>? findUpdatePath({
    required String deviceType,
    required String currentFirmwareVersion,
    required Set<int> supportedProtocols,
  });

  /// Get the file path to a component within an installed package.
  ///
  /// Returns null if the package or component doesn't exist.
  File? componentPath(
    String deviceType,
    String firmwareVersion,
    String componentKey,
  );

  /// Identify packages that can be safely removed.
  ///
  /// A package is cleanable if:
  ///   - No known device is running that firmware version
  ///   - It is not the latest compatible version for any known device type
  List<MaynaPackageManifest> findCleanablePackages(
    List<KnownDevice> knownDevices,
  );

  /// Remove all cleanable packages. Returns the number removed.
  Future<int> cleanup(List<KnownDevice> knownDevices);
}
