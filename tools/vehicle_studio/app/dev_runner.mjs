import { spawn } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import { dirname, resolve } from 'node:path'
import { createServer } from 'vite'

const appDirectory = dirname(fileURLToPath(import.meta.url))
const toolDirectory = resolve(appDirectory, '..')
const repoRoot = resolve(appDirectory, '..', '..', '..')
const options = parseArguments(process.argv.slice(2))
process.env.VEHICLE_STUDIO_API_PORT = String(options.apiPort)

const api = spawn(options.python, [
  '-m', 'vehicle_studio.api_server',
  '--repo-root', repoRoot,
  '--host', '127.0.0.1',
  '--port', String(options.apiPort),
], {
  cwd: toolDirectory,
  stdio: ['ignore', 'inherit', 'inherit'],
  windowsHide: true,
})

let vite
let shuttingDown = false

api.once('exit', (code, signal) => {
  if (!shuttingDown) {
    console.error('La API terminó inesperadamente (code=' + code + ', signal=' + signal + ').')
    void shutdown(1)
  }
})

process.once('SIGINT', () => void shutdown(0))
process.once('SIGTERM', () => void shutdown(0))

try {
  await waitForApi(options.apiPort, api)
  vite = await createServer({
    configFile: resolve(appDirectory, 'vite.config.ts'),
    server: { host: options.host, port: options.port, strictPort: true },
  })
  await vite.listen()
  console.log('API local disponible en http://127.0.0.1:' + options.apiPort)
  vite.printUrls()
  console.log('Presiona Ctrl+C para detener Vue y Python.')
} catch (error) {
  console.error(error instanceof Error ? error.message : error)
  await shutdown(1)
}

async function shutdown(exitCode) {
  if (shuttingDown) return
  shuttingDown = true
  if (vite) await vite.close()
  if (api.exitCode === null && api.signalCode === null) {
    api.kill()
    await Promise.race([
      new Promise((resolveExit) => api.once('exit', resolveExit)),
      new Promise((resolveTimeout) => setTimeout(resolveTimeout, 3000)),
    ])
  }
  process.exitCode = exitCode
}

async function waitForApi(port, child) {
  const url = 'http://127.0.0.1:' + port + '/api/health'
  for (let attempt = 0; attempt < 40; attempt += 1) {
    if (child.exitCode !== null) throw new Error('La API Python terminó antes de estar disponible.')
    try {
      const response = await fetch(url, { signal: AbortSignal.timeout(1000) })
      if (response.ok && (await response.json()).status === 'ok') return
    } catch {
      // The process may still be importing; retry for at most ten seconds.
    }
    await new Promise((resolveWait) => setTimeout(resolveWait, 250))
  }
  throw new Error('La API local no respondió en ' + url)
}

function parseArguments(args) {
  const result = { port: 5173, apiPort: 8765, host: '127.0.0.1', python: 'python' }
  for (let index = 0; index < args.length; index += 2) {
    const value = args[index + 1]
    if (args[index] === '--port') result.port = Number(value)
    else if (args[index] === '--api-port') result.apiPort = Number(value)
    else if (args[index] === '--host') result.host = value
    else if (args[index] === '--python') result.python = value
    else throw new Error('Argumento desconocido: ' + args[index])
  }
  for (const [name, value] of [['port', result.port], ['api-port', result.apiPort]]) {
    if (!Number.isInteger(value) || value < 1 || value > 65535) {
      throw new Error('Puerto inválido: ' + name)
    }
  }
  return result
}
