import 'package:cashurs_wallet/ffi.dart';
import 'package:flutter/material.dart';

class OverviewPage extends StatefulWidget {
  const OverviewPage({super.key});

  @override
  State<OverviewPage> createState() => _OverviewPageState();
}

class _OverviewPageState extends State<OverviewPage> {
  late Future<int> initDB;
  late Future<int> balance;
  @override
  void initState() {
    super.initState();
    initDB = api.initDb();
    balance = api.getBalance();
  }

  @override
  Widget build(BuildContext context) {
    return Container(
        margin: const EdgeInsets.all(24),
        child: Center(
          child: Column(children: [
            FutureBuilder(
                future: Future.wait([initDB, balance]),
                builder: (context, snap) {
                  if (snap.error != null) {
                    // An error has been encountered, so give an appropriate response and
                    // pass the error details to an unobstructive tooltip.
                    debugPrint(snap.error.toString());
                    return Tooltip(
                      message: snap.error.toString(),
                      child: const Text('Error occured'),
                    );
                  }

                  final data = snap.data;
                  if (data == null) return const CircularProgressIndicator();

                  var value = data[1];
                  return Text('$value (sats)',
                      style: const TextStyle(fontSize: 42));
                })
          ]),
        ));
  }
}
