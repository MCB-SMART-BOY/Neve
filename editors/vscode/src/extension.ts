import * as vscode from 'vscode';
import * as path from 'path';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind,
} from 'vscode-languageclient/node';

let client: LanguageClient;

export function activate(context: vscode.ExtensionContext) {
    // The server is a separate `n3v3 lsp` process.
    const serverCommand = 'n3v3';
    const serverArgs = ['lsp'];

    const serverOptions: ServerOptions = {
        command: serverCommand,
        args: serverArgs,
        transport: TransportKind.stdio,
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: 'file', language: 'n3v3' }],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.n3v3'),
        },
    };

    client = new LanguageClient(
        'n3v3-lsp',
        'n3v3 Language Server',
        serverOptions,
        clientOptions,
    );

    client.start();

    vscode.window.showInformationMessage('n3v3 LSP started');
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) {
        return undefined;
    }
    return client.stop();
}
