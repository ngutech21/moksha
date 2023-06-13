import 'package:cashurs_wallet/pages/pay_invoice_page.dart';
import 'package:flutter/material.dart';
import 'package:cashurs_wallet/pages/mint_page.dart';
import 'package:cashurs_wallet/pages/overview_page.dart';
import 'package:cashurs_wallet/pages/receive_page.dart';

void main() {
  runApp(const MyApp());
}

class MyApp extends StatelessWidget {
  const MyApp({Key? key}) : super(key: key);

  // This widget is the root of your application.
  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Flutter Cashu Wallet',
      debugShowCheckedModeBanner: false,
      theme: ThemeData.dark(),
      themeMode: ThemeMode.dark,
      darkTheme: ThemeData(
        brightness: Brightness.dark,
      ),
      home: const MyHomePage(title: 'Home'),
    );
  }
}

class MyHomePage extends StatefulWidget {
  const MyHomePage({Key? key, required this.title}) : super(key: key);

  final String title;

  @override
  State<MyHomePage> createState() => _MyHomePageState();
}

class _MyHomePageState extends State<MyHomePage> {
  int currentIndex = 0;

  @override
  void initState() {
    super.initState();
  }

  List<Widget> createWidget() {
    return <Widget>[
      const Text("Home"),
    ];
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      bottomNavigationBar: NavigationBar(
        selectedIndex: currentIndex,
        onDestinationSelected: (int index) {
          setState(() {
            currentIndex = index;
          });
        },
        destinations: const <Widget>[
          NavigationDestination(
            icon: Icon(Icons.home),
            label: 'Home',
          ),
          NavigationDestination(
            icon: Icon(Icons.currency_bitcoin),
            label: 'Mint',
          ),
          NavigationDestination(
            icon: Icon(Icons.import_export),
            label: 'Receive',
          ),
          NavigationDestination(
            icon: Icon(Icons.bolt),
            label: 'Pay',
          ),
        ],
      ),
      body: <Widget>[
        Container(
          alignment: Alignment.center,
          child: const OverviewPage(),
        ),
        Container(
          alignment: Alignment.center,
          child: const MintPage(),
        ),
        Container(
          alignment: Alignment.center,
          child: const ReceivePage(),
        ),
        Container(
          alignment: Alignment.center,
          child: const PayInvoicePage(),
        ),
      ][currentIndex],
    );
  }
}
