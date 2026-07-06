import 'package:flutter/material.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';

import 'ota_section.dart';

class OtaPage extends StatelessWidget {
  final BluetoothDevice device;

  const OtaPage({super.key, required this.device});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Firmware Update')),
      // OtaSection is a fixed Column of cards that can exceed screen height on
      // small phones; this Scaffold body has no ancestor scroll view, so wrap.
      body: SingleChildScrollView(
        padding: const EdgeInsets.all(16),
        child: OtaSection(device: device),
      ),
    );
  }
}
