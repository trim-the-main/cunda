import 'package:flutter/material.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';

import 'system_status.dart';

class SystemStatusPage extends StatelessWidget {
  final BluetoothDevice device;

  const SystemStatusPage({super.key, required this.device});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text("System Stats")),
      body: SystemStatusSection(device: device),
    );
  }
}
