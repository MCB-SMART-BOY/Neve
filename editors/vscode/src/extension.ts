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
    // The server is a separate `neve lsp` process.
    const serverCommand = 'neve';
    const serverArgs = ['lsp'];

    const serverOptions: ServerOptions = {
        command: serverCommand,
        args: serverArgs,
        transport: TransportKind.stdio,
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: 'file', language: 'neve' }],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.neve'),
        },
    };

    client = new LanguageClient(
        'neve-lsp',
        'Neve Language Server',
        serverOptions,
        clientOptions,
    );

    client.start();

    vscode.window.showInformationMessage('Neve LSP started');
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) {
        return undefined;
    }
    return client.stop();
}
