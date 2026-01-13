const { workspace, commands, window } = require('vscode');
const {
  LanguageClient,
  TransportKind,
} = require('vscode-languageclient/node');
const { spawn } = require('child_process');

/** @type {LanguageClient | undefined} */
let client;

function activate(context) {
  const serverCommand = workspace
    .getConfiguration('chic')
    .get('lsp.path', 'impact-lsp');
  const serverOptions = {
    command: serverCommand,
    transport: TransportKind.stdio,
  };
  const clientOptions = {
    documentSelector: [{ scheme: 'file', language: 'chic' }],
  };

  client = new LanguageClient(
    'chicLsp',
    'Chic LSP',
    serverOptions,
    clientOptions
  );
  context.subscriptions.push(client.start());

  context.subscriptions.push(
    commands.registerCommand('chic.build', () => runChicCommand('build'))
  );
  context.subscriptions.push(
    commands.registerCommand('chic.test', () => runChicCommand('test'))
  );
  context.subscriptions.push(
    commands.registerCommand('chic.run', () => runChicCommand('run'))
  );
}

function runChicCommand(subcommand) {
  const cwd =
    workspace.workspaceFolders && workspace.workspaceFolders.length > 0
      ? workspace.workspaceFolders[0].uri.fsPath
      : process.cwd();
  const cmd = spawn('chic', [subcommand], { cwd });
  const channel = window.createOutputChannel('Chic');
  channel.show(true);
  cmd.stdout.on('data', (data) => channel.append(data.toString()));
  cmd.stderr.on('data', (data) => channel.append(data.toString()));
  cmd.on('close', (code) => {
    channel.appendLine(`chic ${subcommand} exited with code ${code}`);
  });
}

function deactivate() {
  if (client) {
    return client.stop();
  }
  return undefined;
}

module.exports = {
  activate,
  deactivate,
};
