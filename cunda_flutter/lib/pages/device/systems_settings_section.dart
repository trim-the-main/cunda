import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/v1/endpoints.dart';
import 'package:cunda_flutter/providers/rpc/protocol.dart';
import 'package:flutter/material.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

class SystemsSettingsSection extends ConsumerStatefulWidget {
  const SystemsSettingsSection({super.key, required this.device});

  final BluetoothDevice device;

  @override
  ConsumerState<SystemsSettingsSection> createState() =>
      _SystemsSettingsSectionState();
}

class _SystemsSettingsSectionState
    extends ConsumerState<SystemsSettingsSection> {
  final bleDeviceNameController = TextEditingController();

  @override
  void initState() {
    super.initState();
    reload();
  }

  @override
  void dispose() {
    bleDeviceNameController.dispose();
    super.dispose();
  }

  void reload() {
    ref
        .refresh(systemSettingsProvider(widget.device))
        .when(
          data: (settings) {
            bleDeviceNameController.text = settings.bleDeviceName;
          },
          error: (error, stackTrace) {},
          loading: () {},
        );
  }

  void save() {
    final SysSettings newSettings = SysSettings(
      wifiSsid: "",
      wifiPassword: "",
      bleDeviceName: bleDeviceNameController.text,
    );
    ref.read(systemSettingsProvider(widget.device).notifier).save(newSettings);
  }

  @override
  Widget build(BuildContext context) {
    return ref
        .watch(systemSettingsProvider(widget.device))
        .when(
          data: (settings) {
            return Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  "System Settings".toUpperCase(),
                  style: Theme.of(context).textTheme.titleMedium?.copyWith(
                    fontWeight: FontWeight.bold,
                  ),
                ),
                SizedBox(height: 20),

                TextField(
                  controller: bleDeviceNameController,
                  decoration: InputDecoration(
                    labelText: "BLE Device Name",
                    border: OutlineInputBorder(),
                  ),
                ),
                SizedBox(height: 12),
                Center(
                  child: ElevatedButton(
                    onPressed: reload,
                    child: Text("Reload"),
                  ),
                ),
                SizedBox(height: 12),
                Center(
                  child: ElevatedButton(onPressed: save, child: Text("Save")),
                ),
                SizedBox(height: 12),
              ],
            );
          },
          error: (Object error, StackTrace stackTrace) =>
              Text("Error getting system settings"),
          loading: () => Center(child: CircularProgressIndicator()),
        );
  }
}
