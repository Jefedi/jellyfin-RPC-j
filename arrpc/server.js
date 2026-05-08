import RPCServer from 'arrpc';

const server = new RPCServer();

server.on('activity', (data) => {
    const name = data?.activity?.name ?? data?.name ?? 'unknown';
    const appId = data?.activity?.application_id ?? data?.application_id ?? '';
    console.log(`[activity] ${appId} ${name}`);
});

server.on('invite', (data) => {
    console.log('[invite]', data);
});

const shutdown = (signal) => {
    console.log(`\n[arrpc] received ${signal}, exiting`);
    process.exit(0);
};
process.on('SIGTERM', () => shutdown('SIGTERM'));
process.on('SIGINT', () => shutdown('SIGINT'));

console.log('[arrpc] bridge ready');
console.log(`[arrpc] IPC socket dir: ${process.env.XDG_RUNTIME_DIR ?? '/tmp'}`);
console.log('[arrpc] WebSocket on port 1337 for the browser extension');
