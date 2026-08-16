import 'package:flutter/material.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';

import 'system_settings_section.dart';

class SettingsPage extends StatelessWidget {
  final BluetoothDevice device;

  const SettingsPage({super.key, required this.device});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Device Settings')),
      // OtaSection is a fixed Column of cards that can exceed screen height on
      // small phones; this Scaffold body has no ancestor scroll view, so wrap.
      body: SingleChildScrollView(
        padding: const EdgeInsets.all(16),
        child: SystemSettingsSection(device: device),
      ),
    );
  }
}
