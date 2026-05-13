import 'dart:io';

import 'mayna_types.dart';
import 'package_registry.dart';

/// Handles fetching the remote manifest and downloading packages.
abstract class RemotePackageSource {
  /// Fetch the remote manifest from the server.
  /// Caches it locally as remote_manifest.json.
  Future<RemoteManifest> refresh();

  /// Return the cached remote manifest (from last refresh).
  /// Returns null if never fetched.
  RemoteManifest? get cached;

  /// Find packages available remotely for a given device type
  /// that are not installed locally.
  List<RemotePackageInfo> findAvailable({
    required String deviceType,
    required PackageRegistry localRegistry,
  });

  /// Download a package to a temporary file.
  /// Verifies package_sha256 after download.
  /// Returns the temp file for passing to PackageRegistry.import().
  Future<File> download(RemotePackageInfo package);
}
