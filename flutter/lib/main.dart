import 'package:cashurs_wallet/pages/pay_invoice_page.dart';
import 'package:cashurs_wallet/pages/settings_page.dart';
import 'package:flutter/material.dart';
import 'package:cashurs_wallet/pages/mint_page.dart';
import 'package:cashurs_wallet/pages/overview_page.dart';
import 'package:cashurs_wallet/pages/receive_page.dart';
import 'package:cashurs_wallet/ffi.dart';

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
        useMaterial3: true,
        brightness: Brightness.dark,
        colorScheme: const ColorScheme.dark(
          primary: Color.fromARGB(253, 2, 133, 240),
          secondary: Color.fromARGB(253, 2, 133, 240),
          //background: Color.fromARGB(80, 103, 102, 102),
        ),
      ),
      home: const MyHomePage(),
    );
  }
}

class MyHomePage extends StatefulWidget {
  const MyHomePage({Key? key}) : super(key: key);

  @override
  State<MyHomePage> createState() => _MyHomePageState();
}

class _MyHomePageState extends State<MyHomePage> {
  int currentIndex = 0;

  @override
  void initState() {
    super.initState();
    _initCashuWallet();
  }

  Future<void> _initCashuWallet() async {
    await api.initCashu(dbPath: "../data/wallet/cashurs_wallet.db");
  }

  List<Widget> createWidget() {
    return <Widget>[
      const Text("Home"),
    ];
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(toolbarHeight: 50, actions: <Widget>[
        IconButton(
            icon: const Icon(Icons.settings),
            tooltip: 'Settings',
            onPressed: () {
              Navigator.of(context).push(
                MaterialPageRoute(
                  builder: (context) => const SettingsPage(),
                ),
              );
            })
      ]),
      bottomNavigationBar: NavigationBar(
        selectedIndex: currentIndex,
        onDestinationSelected: (int index) {
          setState(() {
            currentIndex = index;
          });
        },
        destinations: const <Widget>[
          NavigationDestination(
            tooltip: '',
            icon: Icon(Icons.home),
            label: 'Home',
          ),
          NavigationDestination(
            tooltip: '',
            icon: Icon(Icons.currency_bitcoin),
            label: 'Mint',
          ),
          NavigationDestination(
            tooltip: '',
            icon: Icon(Icons.import_export),
            label: 'Receive',
          ),
          NavigationDestination(
            tooltip: '',
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
