import 'package:cunda_flutter/pages/device/application_settings_section.dart';
import 'package:cunda_flutter/pages/device/ota_section.dart';
import 'package:cunda_flutter/pages/device/systems_settings_section.dart';
import 'package:flutter/material.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:logging/logging.dart';

final _log = Logger('SettingsTabPage');

class SettingsTabPage extends ConsumerWidget {
  const SettingsTabPage({super.key, required this.device});

  final BluetoothDevice device;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    _log.fine("Building Settings Tab Page");
    final List<Widget> applTiles = [
      Card(
        child: Padding(
          padding: const EdgeInsets.all(8.0),
          child: ApplicationSettingsSection(device: device),
        ),
      ),
      Card(
        child: Padding(
          padding: const EdgeInsets.all(8.0),
          child: SystemsSettingsSection(device: device),
        ),
      ),
      Card(
        child: Padding(
          padding: const EdgeInsets.all(8.0),
          child: OtaSection(device: device),
        ),
      ),
    ];

    return ListView.separated(
      padding: EdgeInsets.all(20),
      itemCount: applTiles.length,
      separatorBuilder: (BuildContext context, int index) =>
          const SizedBox(height: 20),
      itemBuilder: (BuildContext context, int index) {
        return applTiles[index];
      },
    );
  }
}
