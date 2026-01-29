import 'dart:async';
import 'dart:collection';

import 'package:cunda_flutter/providers/rpc/protocol.dart';
import 'package:flutter/material.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:intl/intl.dart';
import 'package:logging/logging.dart';

final _log = Logger('ApplicationTabPage');

class ApplicationTabPage extends StatefulWidget {
  const ApplicationTabPage({super.key, required this.device});

  final BluetoothDevice device;

  @override
  State<ApplicationTabPage> createState() => _ApplicationTabPageState();
}

class _ApplicationTabPageState extends State<ApplicationTabPage>
    with AutomaticKeepAliveClientMixin {
  @override
  bool get wantKeepAlive => true;

  @override
  void initState() {
    super.initState();
    _log.fine("initState of ApplicationTabPage");
  }

  @override
  void dispose() {
    _log.fine("dispose of ApplicationTabPage");
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    super.build(context);
    final List<Widget> applTiles = [
      Card(
        child: Padding(
          padding: const EdgeInsets.all(8.0),
          child: GpioSection(device: widget.device),
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

class GpioSection extends ConsumerStatefulWidget {
  const GpioSection({super.key, required this.device});

  final BluetoothDevice device;

  @override
  ConsumerState<GpioSection> createState() => _GpioSectionState();
}

class _GpioSectionState extends ConsumerState<GpioSection> {
  final StreamController<String> _buttonEventOutput =
      StreamController<String>();

  @override
  void initState() {
    super.initState();
  }

  @override
  void dispose() {
    _log.fine("Disposing GpioSection stateful widget");
    _buttonEventOutput.close();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    _log.fine("Listening to gpio button events stream");
    final StreamController<String> buttonEventOutput =
        StreamController<String>();

    ref.listen(gpioButtonEventsProvider(widget.device), (prev, next) {
      next.whenData((value) {
        final time = DateFormat('Hms').format(value.$1);
        buttonEventOutput.sink.add(
          "$time Pressed for ${value.$2.pressTimeInMs} ms  ",
        );
      });
    });
    return ref
        .watch(endpointDispatcherProvider(widget.device))
        .when(
          data: (eDispatcher) {
            _log.fine("Rebuilding GPIO section stateful widget");
            return Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  "Gpio".toUpperCase(),
                  style: Theme.of(context).textTheme.titleMedium?.copyWith(
                    fontWeight: FontWeight.bold,
                  ),
                ),
                SizedBox(height: 12),
                ConsoleLikeTextField(
                  txtStream: buttonEventOutput.stream,
                  rowCount: 4,
                ),
                SizedBox(height: 12),

                Center(
                  child: ElevatedButton(
                    onPressed: () {
                      eDispatcher.blinkLedEndpoint(req: 1);
                    },
                    child: Text("Blink Once"),
                  ),
                ),
                SizedBox(height: 12),
                Center(
                  child: ElevatedButton(
                    onPressed: () {
                      eDispatcher.blinkLedEndpoint(req: 2);
                    },
                    child: Text("Blink Twice"),
                  ),
                ),
                SizedBox(height: 12),
                Center(
                  child: ElevatedButton(
                    onPressed: () {
                      eDispatcher.blinkLedEndpoint(req: 3);
                    },
                    child: Text("Blink Thrice"),
                  ),
                ),
                SizedBox(height: 12),
              ],
            );
          },
          error: (Object error, StackTrace stackTrace) {
            Navigator.of(context).pop();
            return Text("Error getting endpoint dispatcher");
          },
          loading: () => Center(child: CircularProgressIndicator()),
        );
  }
}

class ConsoleLikeTextField extends StatefulWidget {
  final Stream<String> txtStream;
  final int rowCount;
  static const TextStyle textStyle = TextStyle(
    fontFamily: 'monospace',
    fontSize: 10.0,
  );
  static const InputDecoration decoration = InputDecoration(
    border: OutlineInputBorder(),
    contentPadding: EdgeInsets.all(8.0),
  );
  const ConsoleLikeTextField({
    super.key,
    required this.txtStream,
    required this.rowCount,
  });

  @override
  State<ConsoleLikeTextField> createState() => _ConsoleLikeTextFieldState();
}

class _ConsoleLikeTextFieldState extends State<ConsoleLikeTextField> {
  final TextEditingController _controller = TextEditingController();
  Queue<String> lines = Queue<String>();
  late StreamSubscription textStreamSub;

  void addLine(String line) {
    lines.add(line);
    while (lines.length > widget.rowCount) {
      lines.removeFirst();
    }
    _controller.text =
        lines.join("\n") + "\n" * (widget.rowCount - lines.length);
  }

  @override
  void initState() {
    _log.fine("initState of ConsoleLikeTextField");
    super.initState();
    textStreamSub = widget.txtStream.listen((newline) {
      setState(() {
        addLine(newline);
      });
    });
  }

  @override
  void dispose() {
    _log.fine("dispose of ConsoleLikeTextField");
    textStreamSub.cancel();
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(8.0),
      child: TextField(
        controller: _controller,
        style: ConsoleLikeTextField.textStyle,
        maxLines: widget.rowCount,
        minLines: widget.rowCount,
        readOnly: true,
        decoration: ConsoleLikeTextField.decoration,
      ),
    );
  }
}
