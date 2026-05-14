import 'dart:convert';
import 'dart:io';

import 'package:archive/archive.dart';
import 'package:crypto/crypto.dart';
import 'package:logging/logging.dart';

import 'mayna_types.dart';
import 'package_registry.dart';
import 'version.dart';

final _log = Logger('PackageRegistry');

class PackageRegistryImpl extends PackageRegistry {
  final Directory _baseDir;
  final Map<String, MaynaPackageManifest> _manifests = {};

  PackageRegistryImpl._(this._baseDir);

  static Future<PackageRegistryImpl> init(Directory baseDir) async {
    final registry = PackageRegistryImpl._(baseDir);
    await registry._reconcile();
    return registry;
  }

  Directory get _packagesDir => Directory('${_baseDir.path}/packages');
  File get _localManifestFile => File('${_baseDir.path}/local_manifest.json');
  Directory get _tmpDir => Directory('${_packagesDir.path}/.tmp');

  static String _dirName(String deviceType, String firmwareVersion) =>
      '${deviceType}_$firmwareVersion';

  static String _key(String deviceType, String firmwareVersion) =>
      '$deviceType/$firmwareVersion';

  /// Startup reconciliation:
  ///   1. Delete .tmp/ directory
  ///   2. Load local_manifest.json, dropping entries whose directories are gone
  ///   3. Scan for directories not in the manifest and add them
  Future<void> _reconcile() async {
    if (await _tmpDir.exists()) {
      _log.info('Cleaning up stale temp directory');
      await _tmpDir.delete(recursive: true);
    }

    _manifests.clear();
    if (await _localManifestFile.exists()) {
      final json = jsonDecode(await _localManifestFile.readAsString());
      final packages = json['packages'] as Map<String, dynamic>;
      for (final entry in packages.entries) {
        final pkgJson = entry.value as Map<String, dynamic>;
        final dirName = pkgJson['dir'] as String;
        final manifest = MaynaPackageManifest.fromJson(
          pkgJson['manifest'] as Map<String, dynamic>,
        );
        final dir = Directory('${_packagesDir.path}/$dirName');
        if (await dir.exists()) {
          _manifests[entry.key] = manifest;
        } else {
          _log.warning(
            'Dropping manifest entry ${entry.key}: directory missing',
          );
        }
      }
    }

    if (await _packagesDir.exists()) {
      await for (final entity in _packagesDir.list()) {
        if (entity is! Directory) continue;
        final dirName = entity.uri.pathSegments.where((s) => s.isNotEmpty).last;
        if (dirName == '.tmp') continue;

        final manifestFile = File('${entity.path}/manifest.json');
        if (await manifestFile.exists()) {
          final manifest = MaynaPackageManifest.fromJson(
            jsonDecode(await manifestFile.readAsString()),
          );
          final key = _key(manifest.deviceType, manifest.firmwareVersion);
          if (!_manifests.containsKey(key)) {
            _log.info('Recovering untracked package: $key');
            _manifests[key] = manifest;
          }
        }
      }
    }

    await _saveLocalManifest();
  }

  Future<void> _saveLocalManifest() async {
    final packages = <String, dynamic>{};
    for (final entry in _manifests.entries) {
      final m = entry.value;
      packages[entry.key] = {
        'dir': _dirName(m.deviceType, m.firmwareVersion),
        'manifest': m.toJson(),
      };
    }
    final json = {'packages': packages};
    await _localManifestFile.create(recursive: true);
    await _localManifestFile.writeAsString(jsonEncode(json));
  }

  @override
  Future<MaynaPackageManifest> import(File maynaFile) async {
    final bytes = await maynaFile.readAsBytes();
    final archive = ZipDecoder().decodeBytes(bytes);

    final manifestEntry = archive.findFile('manifest.json');
    if (manifestEntry == null) {
      throw StateError('manifest.json not found in archive');
    }
    final manifest = MaynaPackageManifest.fromJson(
      jsonDecode(utf8.decode(manifestEntry.content as List<int>)),
    );

    _log.info('Importing ${manifest.deviceType}/${manifest.firmwareVersion}');

    if (await _tmpDir.exists()) {
      await _tmpDir.delete(recursive: true);
    }
    await _tmpDir.create(recursive: true);

    try {
      for (final component in manifest.components.values) {
        final archiveFile = archive.findFile(component.file);
        if (archiveFile == null) {
          throw StateError('Component ${component.file} not found in archive');
        }
        final data = archiveFile.content as List<int>;

        if (data.length != component.size) {
          throw StateError(
            'Size mismatch for ${component.file}: '
            'expected ${component.size}, got ${data.length}',
          );
        }

        final hash = sha256.convert(data).toString();
        if (hash != component.sha256) {
          throw StateError(
            'SHA256 mismatch for ${component.file}: '
            'expected ${component.sha256}, got $hash',
          );
        }

        await File('${_tmpDir.path}/${component.file}').writeAsBytes(data);
      }

      // Write manifest.json last — its presence signals a complete extraction
      await File(
        '${_tmpDir.path}/manifest.json',
      ).writeAsString(jsonEncode(manifest.toJson()));

      final dirName = _dirName(manifest.deviceType, manifest.firmwareVersion);
      final targetDir = Directory('${_packagesDir.path}/$dirName');
      if (await targetDir.exists()) {
        await targetDir.delete(recursive: true);
      }
      await _tmpDir.rename(targetDir.path);

      final key = _key(manifest.deviceType, manifest.firmwareVersion);
      _manifests[key] = manifest;
      await _saveLocalManifest();

      _log.info('Imported ${manifest.deviceType}/${manifest.firmwareVersion}');
      return manifest;
    } catch (e) {
      _log.severe('Import failed, cleaning up: $e');
      if (await _tmpDir.exists()) {
        await _tmpDir.delete(recursive: true);
      }
      rethrow;
    }
  }

  @override
  Future<void> remove(String deviceType, String firmwareVersion) async {
    final key = _key(deviceType, firmwareVersion);
    _manifests.remove(key);

    final dir = Directory(
      '${_packagesDir.path}/${_dirName(deviceType, firmwareVersion)}',
    );
    if (await dir.exists()) {
      await dir.delete(recursive: true);
    }
    await _saveLocalManifest();
    _log.info('Removed $key');
  }

  @override
  List<MaynaPackageManifest> listAll() => _manifests.values.toList();

  @override
  List<MaynaPackageManifest> listByDeviceType(String deviceType) =>
      _manifests.values.where((m) => m.deviceType == deviceType).toList();

  @override
  MaynaPackageManifest? find(String deviceType, String firmwareVersion) =>
      _manifests[_key(deviceType, firmwareVersion)];

  @override
  MaynaPackageManifest? findLatestCompatible({
    required String deviceType,
    required String currentFirmwareVersion,
    required Set<int> supportedProtocols,
  }) {
    final candidates = listByDeviceType(deviceType).where((m) {
      if (!supportedProtocols.contains(m.protocolVersion)) return false;
      if (compareVersions(m.minFirmwareVersion, currentFirmwareVersion) > 0) {
        return false;
      }
      return true;
    }).toList();

    if (candidates.isEmpty) return null;

    candidates.sort(
      (a, b) => compareVersions(a.firmwareVersion, b.firmwareVersion),
    );
    return candidates.last;
  }

  @override
  List<MaynaPackageManifest>? findUpdatePath({
    required String deviceType,
    required String currentFirmwareVersion,
    required Set<int> supportedProtocols,
  }) {
    final allVersions = listByDeviceType(
      deviceType,
    ).where((m) => supportedProtocols.contains(m.protocolVersion)).toList();
    allVersions.sort(
      (a, b) => compareVersions(a.firmwareVersion, b.firmwareVersion),
    );

    final path = <MaynaPackageManifest>[];
    var current = currentFirmwareVersion;

    for (final pkg in allVersions) {
      if (compareVersions(pkg.firmwareVersion, current) <= 0) continue;
      if (compareVersions(pkg.minFirmwareVersion, current) <= 0) {
        path.add(pkg);
        current = pkg.firmwareVersion;
      }
    }

    return path.isEmpty ? null : path;
  }

  @override
  File? componentPath(
    String deviceType,
    String firmwareVersion,
    ComponentType componentType,
  ) {
    final manifest = find(deviceType, firmwareVersion);
    if (manifest == null) return null;

    final component = manifest.components[componentType];
    if (component == null) return null;

    final dirName = _dirName(deviceType, firmwareVersion);
    final file = File('${_packagesDir.path}/$dirName/${component.file}');
    return file.existsSync() ? file : null;
  }

  @override
  List<MaynaPackageManifest> findCleanablePackages(
    List<KnownDevice> knownDevices,
  ) {
    final activeVersionsByType = <String, Set<String>>{};
    for (final device in knownDevices) {
      activeVersionsByType
          .putIfAbsent(device.deviceType, () => {})
          .add(device.firmwareVersion);
    }

    // Retain the latest version per device type
    final latestPerType = <String, String>{};
    for (final deviceType
        in _manifests.values.map((m) => m.deviceType).toSet()) {
      final sorted = listByDeviceType(deviceType).toList()
        ..sort((a, b) => compareVersions(a.firmwareVersion, b.firmwareVersion));
      if (sorted.isNotEmpty) {
        latestPerType[deviceType] = sorted.last.firmwareVersion;
      }
    }

    return _manifests.values.where((m) {
      // Keep if a known device is running this version
      final activeVersions = activeVersionsByType[m.deviceType];
      if (activeVersions != null &&
          activeVersions.contains(m.firmwareVersion)) {
        return false;
      }

      // Keep if it's the latest for its device type
      if (latestPerType[m.deviceType] == m.firmwareVersion) {
        return false;
      }

      return true;
    }).toList();
  }

  @override
  Future<int> cleanup(List<KnownDevice> knownDevices) async {
    final cleanable = findCleanablePackages(knownDevices);
    for (final pkg in cleanable) {
      await remove(pkg.deviceType, pkg.firmwareVersion);
    }
    return cleanable.length;
  }
}
