import 'package:cunda_flutter/pages/devices/nokta/gps_page.dart';
import 'package:cunda_flutter/pages/system/cunda_common_page_components.dart';
import 'package:flutter/material.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';

class NoktaDevicePage extends StatelessWidget {
  final BluetoothDevice device;
  const NoktaDevicePage({super.key, required this.device});

  @override
  Widget build(BuildContext context) {
    return DefaultTabController(
      length: 1,
      child: Scaffold(
        appBar: CundaDeviceAppBar(
          device: device,
          dotMenuActions: [
            CundaCommonAction.logs,
            CundaCommonAction.ota,
            CundaCommonAction.settings,
            CundaCommonAction.sysStats,
          ],
          tabBar: const TabBar(tabs: [Tab(text: "GPS")]),
        ),
        body: TabBarView(children: [GpsPage(device: device)]),
      ),
    );
  }
}
