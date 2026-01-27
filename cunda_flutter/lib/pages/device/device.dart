import 'package:cunda_flutter/utils/bluetooth_device_extension.dart';
import 'package:flutter/material.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';

class DevicePage extends StatelessWidget {
  final BluetoothDevice device;

  const DevicePage({super.key, required this.device});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: Text(device.chosenName), centerTitle: true),
      body: Center(child: Text("Device Page for ${device.chosenName}")),
    );
  }
}
