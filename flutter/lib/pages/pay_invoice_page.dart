// ignore_for_file: use_build_context_synchronously

import 'package:moksha_wallet/ffi.dart';
import 'package:flutter/material.dart';
import 'package:moksha_wallet/pages/util.dart';

class PayInvoicePage extends StatefulWidget {
  const PayInvoicePage({super.key});

  @override
  State<PayInvoicePage> createState() => _PayInvoicePageState();
}

class _PayInvoicePageState extends State<PayInvoicePage> {
  String invoice = '';

  @override
  Widget build(BuildContext context) {
    return Container(
      margin: const EdgeInsets.all(24),
      child: Center(
          child: Column(
        children: [
          TextField(
            obscureText: false,
            maxLines: 2,
            autofocus: true,
            onChanged: (value) => setState(() {
              invoice = value;
            }),
            decoration: const InputDecoration(
              border: OutlineInputBorder(),
              labelText: 'Paste invoice',
            ),
          ),
          const Spacer(),
          ElevatedButton(
              onPressed: () async {
                try {
                  var paid = await api.payInvoice(invoice: invoice);
                  if (!context.mounted) return;
                  ScaffoldMessenger.of(context).showSnackBar(SnackBar(
                    content: Column(children: [
                      paid
                          ? const Text(
                              'Invoice has been paid: Tokens melted successfully')
                          : const Text('Error paying invoice')
                    ]),
                    showCloseIcon: true,
                  ));
                } catch (e) {
                  if (!context.mounted) return;
                  showErrorSnackBar(context, e, 'Error paying invoice');
                }
              },
              child: const Text("Pay invoice"))
        ],
      )),
    );
  }
}
