import 'package:go_router/go_router.dart';
import 'package:flutter/material.dart';
import 'package:moksha_wallet/pages/mint_page.dart';
import 'package:moksha_wallet/pages/overview_page.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:moksha_wallet/pages/pay_invoice_page.dart';
import 'package:moksha_wallet/pages/receive_page.dart';
import 'package:moksha_wallet/pages/settings_page.dart';
import 'package:moksha_wallet/pages/util.dart';

import '../generated/ffi.io.dart' if (dart.library.html) '../generated/ffi.web.dart';
export '../generated/ffi.io.dart' if (dart.library.html) '../generated/ffi.web.dart' show api;

final dbPathProvider = FutureProvider<String>((ref) async {
  return await api.initCashu();
});

final cashuBalanceProvider = StateProvider((ref) => 0);
final fedimintBalanceProvider = StateProvider((ref) => 0);
final btcPriceProvider = StateProvider((ref) => 0.0);

final _rootNavigatorKey = GlobalKey<NavigatorState>();
final _keyHome = GlobalKey<NavigatorState>(debugLabel: 'shellHome');
final _keyMint = GlobalKey<NavigatorState>(debugLabel: 'shellMint');

final _keyReceive = GlobalKey<NavigatorState>(debugLabel: 'shellReceive');
final _keyPay = GlobalKey<NavigatorState>(debugLabel: 'shellPay');
final _keySettings = GlobalKey<NavigatorState>(debugLabel: 'shellSettings');

final goRouter = GoRouter(initialLocation: '/home', navigatorKey: _rootNavigatorKey, debugLogDiagnostics: true, routes: [
  StatefulShellRoute.indexedStack(
      builder: (context, state, navigationShell) {
        return ScaffoldWithNestedNavigation(navigationShell: navigationShell);
      },
      branches: [
        StatefulShellBranch(navigatorKey: _keyHome, routes: [
          GoRoute(
              path: '/home',
              pageBuilder: (context, state) => const NoTransitionPage(
                    child: OverviewPage(label: 'Home'),
                  )),
        ]),
        StatefulShellBranch(navigatorKey: _keyMint, routes: [
          GoRoute(
              path: '/mint',
              pageBuilder: (context, state) => const NoTransitionPage(
                    child: MintPage(),
                  )),
        ]),
        StatefulShellBranch(navigatorKey: _keyReceive, routes: [
          GoRoute(
              path: '/receive',
              pageBuilder: (context, state) => const NoTransitionPage(
                    child: ReceivePage(),
                  )),
        ]),
        StatefulShellBranch(navigatorKey: _keyPay, routes: [
          GoRoute(
              path: '/pay',
              pageBuilder: (context, state) => const NoTransitionPage(
                    child: PayInvoicePage(),
                  )),
        ]),
        StatefulShellBranch(navigatorKey: _keySettings, routes: [
          GoRoute(
              path: '/settings',
              pageBuilder: (context, state) => const NoTransitionPage(
                    child: SettingsPage(),
                  ))
        ]),
      ]),
]);

class ScaffoldWithNestedNavigation extends StatelessWidget {
  const ScaffoldWithNestedNavigation({
    Key? key,
    required this.navigationShell,
  }) : super(key: key ?? const ValueKey<String>('ScaffoldWithNestedNavigation'));
  final StatefulNavigationShell navigationShell;

  void _goBranch(int index) {
    navigationShell.goBranch(
      index,
      initialLocation: index == navigationShell.currentIndex,
    );
  }

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(builder: (context, constraints) {
      return ScaffoldWithNavigationBar(
        body: navigationShell,
        selectedIndex: navigationShell.currentIndex,
        onDestinationSelected: _goBranch,
      );
    });
  }
}

class ScaffoldWithNavigationBar extends StatelessWidget {
  const ScaffoldWithNavigationBar({
    super.key,
    required this.body,
    required this.selectedIndex,
    required this.onDestinationSelected,
  });
  final Widget body;
  final int selectedIndex;
  final ValueChanged<int> onDestinationSelected;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: body,
      bottomNavigationBar: NavigationBar(
        selectedIndex: selectedIndex,
        destinations: const [
          NavigationDestination(label: 'Home', tooltip: '', icon: Icon(Icons.home)),
          NavigationDestination(label: 'Mint', tooltip: '', icon: Icon(Icons.currency_bitcoin)),
          NavigationDestination(label: 'Receive', tooltip: '', icon: Icon(Icons.import_export)),
          NavigationDestination(label: 'Pay', tooltip: '', icon: Icon(Icons.bolt)),
          Visibility(visible: true, child: NavigationDestination(label: 'Settings', tooltip: '', icon: Icon(Icons.settings)))
        ],
        onDestinationSelected: onDestinationSelected,
      ),
    );
  }
}

void main() {
  ErrorWidget.builder = (FlutterErrorDetails details) {
    return MaterialApp(
      home: Scaffold(
        body: Center(
          child: Container(
            alignment: Alignment.center,
            width: 250,
            height: 200,
            decoration: BoxDecoration(
              borderRadius: BorderRadius.circular(10),
              color: Colors.amber[300],
              boxShadow: const [
                BoxShadow(color: Colors.green, spreadRadius: 3),
              ],
            ),
            child: Text(
              ' Error!\n ${details.exception}',
              style: const TextStyle(color: Colors.red, fontSize: 20),
              textAlign: TextAlign.center,
              textDirection: TextDirection.ltr,
            ),
          ),
        ),
      ),
    );
  };

  WidgetsFlutterBinding.ensureInitialized();

  runApp(
    const ProviderScope(
      child: MyApp(),
    ),
  );
}

class MyApp extends ConsumerWidget {
  const MyApp({Key? key}) : super(key: key);

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    ref.watch(dbPathProvider); // this is a hack to trigger the provider

    updateCashuBalance(ref);
    updateFedimintBalance(ref);
    updateBtcPrice(ref);

    return MaterialApp.router(
      title: 'Moksha e-cash Wallet',
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
      routerConfig: goRouter,
    );
  }
}
