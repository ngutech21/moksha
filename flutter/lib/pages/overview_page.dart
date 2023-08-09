import 'package:flutter/material.dart';
import 'package:fl_chart/fl_chart.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:moksha_wallet/main.dart';
import 'package:moksha_wallet/pages/util.dart';
import 'package:go_router/go_router.dart';

class OverviewPage extends ConsumerWidget {
  const OverviewPage({required this.label, Key? key}) : super(key: key);

  final String label;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    var cashuBalance = ref.watch(cashuBalanceProvider);
    var fedimintBalance = ref.watch(fedimintBalanceProvider);
    var btcPriceUsd = ref.watch(btcPriceProvider);

    final totalBalance = cashuBalance + fedimintBalance;
    final formattedTotalBalance = formatSats(totalBalance);

    return Container(
        margin: const EdgeInsets.all(24),
        child: Center(
          child: Column(
            children: [
              Text('$formattedTotalBalance sats', style: const TextStyle(fontSize: 42)),
              Text('${(totalBalance * (btcPriceUsd / 100000000)).toStringAsFixed(2)} \$', style: const TextStyle(fontSize: 32)),
              SizedBox(
                  height: 300.0,
                  width: 300.0,
                  child: PieChart(
                    PieChartData(
                      sections: showingSections(cashuBalance: cashuBalance, fedimintBalance: fedimintBalance), // Required
                    ),
                    swapAnimationDuration: const Duration(milliseconds: 150), // Optional
                    swapAnimationCurve: Curves.linear, // Optional
                  )),
              ElevatedButton(
                  onPressed: () {
                    context.go("/scan");
                  },
                  child: const Text("Scan"))
            ],
          ),
        ));
  }
}

List<PieChartSectionData> showingSections({cashuBalance = int, fedimintBalance = int}) {
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
          borderSide: isTouched ? const BorderSide(color: Colors.white, width: 6) : const BorderSide(color: Colors.black),
        );
      case 1:
        return PieChartSectionData(
          color: Colors.blue,
          value: (fedimintBalance.toDouble() / totalBalance.toDouble()) * 360,
          title: 'Fedimint',
          radius: 65,
          titlePositionPercentageOffset: 0.55,
          borderSide: isTouched ? const BorderSide(color: Colors.white, width: 6) : const BorderSide(color: Colors.black),
        );

      default:
        throw Error();
    }
  });
}
