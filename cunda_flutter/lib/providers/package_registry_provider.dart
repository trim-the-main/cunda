import 'dart:io';

import 'package:cunda_flutter/services/mayna/package_registry.dart';
import 'package:cunda_flutter/utils/riverpod_utils.dart';
import 'package:logging/logging.dart';
import 'package:path_provider/path_provider.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

part 'package_registry_provider.g.dart';

final _log = Logger('PackageRegistryProvider');

@Riverpod(keepAlive: true, retry: noRetry)
Future<PackageRegistry> packageRegistry(Ref ref) async {
  final appDir = await getApplicationDocumentsDirectory();
  _log.info('Loading mayna package registry from ${appDir.path}/mayna');
  return PackageRegistry.init(Directory('${appDir.path}/mayna'));
}
