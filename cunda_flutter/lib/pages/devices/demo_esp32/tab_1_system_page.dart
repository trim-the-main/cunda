import 'package:cunda_flutter/pages/system/system_status.dart';
import 'package:flutter/material.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:logging/logging.dart';

final _log = Logger('SystemTabPage');

class SystemTabPage extends StatelessWidget {
  const SystemTabPage({super.key, required this.device});

  final BluetoothDevice device;

  @override
  Widget build(BuildContext context) {
    _log.fine("Building status tab page");
    return SystemStatusSection(device: device);
  }
}
