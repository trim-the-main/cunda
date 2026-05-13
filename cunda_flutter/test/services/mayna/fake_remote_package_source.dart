import 'dart:io';

import 'package:cunda_flutter/services/mayna/mayna_types.dart';
import 'package:cunda_flutter/services/mayna/package_registry.dart';
import 'package:cunda_flutter/services/mayna/remote_package_source.dart';

/// A fake [RemotePackageSource] backed by in-memory data for testing.
class FakeRemotePackageSource extends RemotePackageSource {
  final List<RemotePackageInfo> remotePackages;
  RemoteManifest? _cached;
  int _nextSequence;

  FakeRemotePackageSource({required this.remotePackages, int initialSequence = 1})
      : _nextSequence = initialSequence;

  @override
  Future<RemoteManifest> refresh() async {
    _cached = RemoteManifest(
      manifestVersion: 1,
      sequenceNumber: _nextSequence++,
      packages: remotePackages,
    );
    return _cached!;
  }

  @override
  RemoteManifest? get cached => _cached;

  @override
  List<RemotePackageInfo> findAvailable({
    required String deviceType,
    required PackageRegistry localRegistry,
  }) {
    if (_cached == null) return [];

    return _cached!.packages.where((pkg) {
      if (pkg.deviceType != deviceType) return false;
      // Exclude if already installed locally
      if (localRegistry.find(pkg.deviceType, pkg.firmwareVersion) != null) {
        return false;
      }
      return true;
    }).toList();
  }

  @override
  Future<File> download(RemotePackageInfo package) async {
    // Create a dummy file for testing
    final tempDir = await Directory.systemTemp.createTemp('mayna_dl_');
    final file = File(
        '${tempDir.path}/${package.deviceType}_${package.firmwareVersion}.mayna');
    await file.writeAsBytes([0x50, 0x4B, 0x03, 0x04]); // ZIP magic bytes
    return file;
  }
}
