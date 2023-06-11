import 'package:flutter/material.dart';

class OverviewPage extends StatelessWidget {
  const OverviewPage({super.key});

  @override
  Widget build(BuildContext context) {
    return Container(
        margin: const EdgeInsets.all(24),
        child: const Center(
          child: Column(children: [
            Text("420 69 (sats)", style: TextStyle(fontSize: 42))
          ]),
        ));
  }
}
