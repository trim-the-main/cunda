import 'dart:convert';
import 'dart:io';

import 'package:crypto/crypto.dart';
import 'package:logging/logging.dart';

import 'mayna_types.dart';
import 'package_registry.dart';
import 'remote_package_source.dart';

final _log = Logger('RemotePackageSource');

class RemotePackageSourceImpl extends RemotePackageSource {
  final Uri _manifestUrl;
  final File _cacheFile;
  final HttpClient _httpClient;
  RemoteManifest? _cached;

  RemotePackageSourceImpl(this._manifestUrl, this._cacheFile)
      : _httpClient = HttpClient();

  @override
  Future<RemoteManifest> refresh() async {
    _log.info('Fetching remote manifest from $_manifestUrl');
    final request = await _httpClient.getUrl(_manifestUrl);
    final response = await request.close();

    if (response.statusCode != 200) {
      throw StateError(
          'Failed to fetch remote manifest: HTTP ${response.statusCode}');
    }

    final body = await response.transform(utf8.decoder).join();
    final json = jsonDecode(body) as Map<String, dynamic>;
    final manifest = RemoteManifest.fromJson(json);

    // Cache to disk
    await _cacheFile.create(recursive: true);
    await _cacheFile.writeAsString(body);

    final previousSeq = _cached?.sequenceNumber;
    _cached = manifest;

    if (previousSeq != null &&
        manifest.sequenceNumber > previousSeq + 1) {
      _log.warning(
          'Sequence gap: expected ${previousSeq + 1}, got ${manifest.sequenceNumber}');
    }

    _log.info(
        'Remote manifest refreshed: seq=${manifest.sequenceNumber}, '
        '${manifest.packages.length} packages');
    return manifest;
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
      if (localRegistry.find(pkg.deviceType, pkg.firmwareVersion) != null) {
        return false;
      }
      return true;
    }).toList();
  }

  @override
  Future<File> download(RemotePackageInfo package) async {
    _log.info(
        'Downloading ${package.deviceType}/${package.firmwareVersion} '
        'from ${package.packageUrl}');

    final request = await _httpClient.getUrl(package.packageUrl);
    final response = await request.close();

    if (response.statusCode != 200) {
      throw StateError(
          'Failed to download package: HTTP ${response.statusCode}');
    }

    final tempDir = await Directory.systemTemp.createTemp('mayna_dl_');
    final file = File(
        '${tempDir.path}/${package.deviceType}_${package.firmwareVersion}.mayna');

    await response.pipe(file.openWrite());

    final bytes = await file.readAsBytes();
    final hash = sha256.convert(bytes).toString();

    if (hash != package.packageSha256) {
      await file.delete();
      await tempDir.delete();
      throw StateError(
          'SHA256 mismatch for downloaded package: '
          'expected ${package.packageSha256}, got $hash');
    }

    _log.info(
        'Downloaded ${package.deviceType}/${package.firmwareVersion} '
        '(${bytes.length} bytes, SHA256 verified)');
    return file;
  }
}
