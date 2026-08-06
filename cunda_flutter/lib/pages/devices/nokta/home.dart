import 'package:cunda_flutter/pages/devices/nokta/gps_page.dart';
import 'package:cunda_flutter/pages/system/cunda_common_page_components.dart';
import 'package:cunda_flutter/pages/system/system_status.dart';
import 'package:flutter/material.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';

class NoktaDevicePage extends StatelessWidget {
  final BluetoothDevice device;
  const NoktaDevicePage({super.key, required this.device});

  @override
  Widget build(BuildContext context) {
    return DefaultTabController(
      length: 2,
      child: Scaffold(
        appBar: CundaDeviceAppBar(
          device: device,
          dotMenuActions: [CundaCommonAction.logs, CundaCommonAction.ota],
          tabBar: const TabBar(
            tabs: [
              Tab(text: "Application"),
              Tab(text: "System"),
            ],
          ),
        ),
        body: TabBarView(
          children: [
            GpsPage(device: device),
            SystemStatus(device: device),
          ],
        ),
      ),
    );
  }
}
