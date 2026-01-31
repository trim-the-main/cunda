import 'package:cunda_flutter/providers/ble/ble_providers.dart';
import 'package:cunda_flutter/services/ble/ble.dart';
import 'package:cunda_flutter/utils/bluetooth_device_extension.dart';
import 'package:cunda_flutter/utils/logging.dart';
import 'package:flutter/material.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:cunda_flutter/pages/device/device.dart';
import 'package:logging/logging.dart';

import 'package:mockito/annotations.dart';
import 'package:mockito/mockito.dart';

@GenerateNiceMocks([MockSpec<BluetoothDevice>(), MockSpec<BleService>()])
import 'device_test.mocks.dart';

final _log = Logger('DevicePageTest');
void main() {
  initializeLogging(level: Level.FINE);
  group('Device Page Widget Tests', () {
    testWidgets('Device page shows green dot when all is good', (
      WidgetTester tester,
    ) async {
      final mockDevice = MockBluetoothDevice();
      when(mockDevice.chosenName).thenReturn("Mock Device");
      when(mockDevice.isConnected).thenReturn(true);
      when(mockDevice.isDisconnected).thenReturn(false);
      // mockDevice.setActionDelay(Duration()); // No delay for testing

      final mockBleService = MockBleService();
      when(mockBleService.adapterStateNow).thenReturn(true);
      when(
        mockBleService.adapterState,
      ).thenAnswer((_) => Stream.fromIterable([BluetoothAdapterState.on]));

      await tester.pumpWidget(
        ProviderScope(
          overrides: [bleServiceProvider.overrideWithValue(mockBleService)],
          child: MaterialApp(home: DevicePage(device: mockDevice)),
        ),
      );
      _log.info("Pumped widget");
      logInvocations([mockBleService]);

      // Verify green dot indicator is displayed
      expect(
        find.byWidgetPredicate(
          (widget) =>
              widget is Container &&
              widget.decoration is BoxDecoration &&
              (widget.decoration as BoxDecoration).color == Colors.green,
        ),
        findsOneWidget,
      );
    });

    testWidgets('Device page with a disconnected device', (
      WidgetTester tester,
    ) async {
      final mockDevice = MockBluetoothDevice();
      when(mockDevice.chosenName).thenReturn("Mock Device");
      when(mockDevice.isConnected).thenReturn(false);
      when(mockDevice.isDisconnected).thenReturn(true);

      when(mockDevice.connectionState).thenAnswer(
        (_) => Stream.fromIterable([BluetoothConnectionState.disconnected]),
      );

      final mockBleService = MockBleService();
      when(mockBleService.adapterStateNow).thenReturn(true);
      when(
        mockBleService.adapterState,
      ).thenAnswer((_) => Stream.fromIterable([BluetoothAdapterState.on]));

      await tester.pumpWidget(
        ProviderScope(
          overrides: [bleServiceProvider.overrideWithValue(mockBleService)],
          child: MaterialApp(home: DevicePage(device: mockDevice)),
        ),
      );
      await tester.pumpAndSettle();

      logInvocations([mockDevice, mockBleService]);

      // Verify red dot indicator is displayed
      expect(
        find.byWidgetPredicate(
          (widget) =>
              widget is Container &&
              widget.decoration is BoxDecoration &&
              (widget.decoration as BoxDecoration).color == Colors.red,
        ),
        findsOneWidget,
      );
    });
  });
}
