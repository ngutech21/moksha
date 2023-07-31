import 'package:async/async.dart';
import 'package:flutter/material.dart';
import 'package:fl_chart/fl_chart.dart';
import 'package:moksha_wallet/pages/util.dart';
import 'package:go_router/go_router.dart';
import '../ffi.io.dart' if (dart.library.html) '../ffi.web.dart';

class OverviewPage extends StatefulWidget {
  const OverviewPage({required this.label, Key? key}) : super(key: key);

  final String label;

  @override
  State<OverviewPage> createState() => _OverviewPageState();
}

class _OverviewPageState extends State<OverviewPage> {
  late Stream<int> cashuBalance;
  late Stream<int> fedimintBalance;
  // late Future<double> btcPrice;
  @override
  void initState() {
    super.initState();
    try {
      cashuBalance = api.getCashuBalance();
      fedimintBalance = api.getFedimintBalance();
      // btcPrice = api.getBtcprice();
    } catch (e) {
      Future<void>.delayed(Duration.zero, () {
        showErrorSnackBar(context, e, 'Error fetching data');
      });
    }
  }

  Stream<List<dynamic>> getCombinedStream() {
    final cashuBalanceStream = api.getCashuBalance();
    final fedimintBalanceStream = api.getFedimintBalance();

    return StreamZip([cashuBalanceStream, fedimintBalanceStream]);
  }

  @override
  Widget build(BuildContext context) {
    return Container(
        margin: const EdgeInsets.all(24),
        child: Center(
          child: Column(children: [
            StreamBuilder(
                stream: getCombinedStream(),
                builder: (context, snap) {
                  if (snap.error != null) {
                    debugPrint(snap.error.toString());
                    return Text(
                      "Error occured:${snap.error}",
                    );
                  }

                  final data = snap.data;
                  if (data == null) return const CircularProgressIndicator();

                  var cashuBalance = data[0];
                  var fedimintBalance = data[1];
                  //var btcPriceUsd = data[2]; // FIXME
                  var btcPriceUsd = 30000.0;
                  var pricePerSat = btcPriceUsd / 100000000;
                  var totalBalance = cashuBalance + fedimintBalance;

                  final regExSeparator = RegExp(r'(\d{1,3})(?=(\d{3})+(?!\d))');
                  matchFunc(Match match) => '${match[1]},';
                  var formattedValue = totalBalance
                      .toString()
                      .replaceAll(',', '')
                      .replaceAllMapped(regExSeparator, matchFunc);

                  return Column(
                    children: [
                      Text('$formattedValue sats',
                          style: const TextStyle(fontSize: 42)),
                      Text(
                          '${(totalBalance * pricePerSat).toStringAsFixed(2)} \$',
                          style: const TextStyle(fontSize: 32)),
                      SizedBox(
                          height: 300.0,
                          width: 300.0,
                          child: PieChart(
                            PieChartData(
                              sections: showingSections(
                                  cashuBalance: cashuBalance,
                                  fedimintBalance: fedimintBalance), // Required
                            ),
                            swapAnimationDuration:
                                const Duration(milliseconds: 150), // Optional
                            swapAnimationCurve: Curves.linear, // Optional
                          )),
                      ElevatedButton(
                          onPressed: () {
                            context.go("/pay");
                          },
                          child: const Text("Pay"))
                    ],
                  );
                })
          ]),
        ));
  }
}

List<PieChartSectionData> showingSections(
    {cashuBalance = int, fedimintBalance = int}) {
  var totalBalance = cashuBalance + fedimintBalance;

  if (totalBalance == 0) {
    return [];
  }

  return List.generate(2, (i) {
    final isTouched = i == 0; // FIXME

    switch (i) {
      case 0:
        return PieChartSectionData(
          color: Colors.pink,
          value: (cashuBalance.toDouble() / totalBalance.toDouble()) * 360,
          title: 'Cashu',
          radius: 80,
          titlePositionPercentageOffset: 0.55,
          borderSide: isTouched
              ? const BorderSide(color: Colors.white, width: 6)
              : const BorderSide(color: Colors.black),
        );
      case 1:
        return PieChartSectionData(
          color: Colors.blue,
          value: (fedimintBalance.toDouble() / totalBalance.toDouble()) * 360,
          title: 'Fedimint',
          radius: 65,
          titlePositionPercentageOffset: 0.55,
          borderSide: isTouched
              ? const BorderSide(color: Colors.white, width: 6)
              : const BorderSide(color: Colors.black),
        );

      default:
        throw Error();
    }
  });
}
