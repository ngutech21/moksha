import 'package:moksha_wallet/ffi.dart';
import 'package:flutter/material.dart';
import 'package:fl_chart/fl_chart.dart';

class OverviewPage extends StatefulWidget {
  const OverviewPage({super.key});

  @override
  State<OverviewPage> createState() => _OverviewPageState();
}

class _OverviewPageState extends State<OverviewPage> {
  late Future<int> balance;
  @override
  void initState() {
    super.initState();
    balance = api.getCashuBalance();
  }

  @override
  Widget build(BuildContext context) {
    return Container(
        margin: const EdgeInsets.all(24),
        child: Center(
          child: Column(children: [
            FutureBuilder(
                future: Future.wait([balance]),
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

                  var value = data[0];

                  final regExSeparator = RegExp(r'(\d{1,3})(?=(\d{3})+(?!\d))');
                  matchFunc(Match match) => '${match[1]},';
                  var formattedValue = value
                      .toString()
                      .replaceAll(',', '')
                      .replaceAllMapped(regExSeparator, matchFunc);

                  return Column(
                    children: [
                      Text('$formattedValue (sats)',
                          style: const TextStyle(fontSize: 42)),
                      SizedBox(
                          height: 300.0,
                          width: 300.0,
                          child: PieChart(
                            PieChartData(
                              sections: showingSections(),
                            ),
                            swapAnimationDuration:
                                const Duration(milliseconds: 150), // Optional
                            swapAnimationCurve: Curves.linear, // Optional
                          ))
                    ],
                  );
                })
          ]),
        ));
  }
}

List<PieChartSectionData> showingSections() {
  return List.generate(2, (i) {
    final isTouched = i == 0;
    const color0 = Colors.blue;
    const color1 = Colors.pink;

    switch (i) {
      case 0:
        return PieChartSectionData(
          color: color0,
          value: 270,
          title: 'Cashu',
          radius: 80,
          titlePositionPercentageOffset: 0.55,
          borderSide: isTouched
              ? const BorderSide(color: Colors.white, width: 6)
              : const BorderSide(color: Colors.black),
        );
      case 1:
        return PieChartSectionData(
          color: color1,
          value: 90,
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
