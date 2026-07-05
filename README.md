# tunfetch

⚡ Blazingly fast drop-in `fetch()` replacement for [Cloudflare Workers](https://workers.cloudflare.com/) with SOCKS5 and HTTP(S) proxy support built with Rust and WASM 🦀

## 📦 Install

```sh
npm install tunfetch-wasm
```

## 🚀 Usage

```js
import { tunfetch } from "tunfetch-wasm";

export default {
  async fetch(request, env) {
    const res = await tunfetch("https://example.com", {
      proxy: "socks5://user:pass@proxy.example.com:1080",
    });

    return res;
  },
};
```

`tunfetch` works identically to the standard `fetch()` the only difference is the additional `proxy` option.

### 🔌 Supported proxy schemes

| Scheme | Example |
| --- | --- |
| `socks5` | `socks5://user:pass@host:port` |
| `http` | `http://user:pass@host:port` |
| `https` | `https://user:pass@host:port` |

## ⚙️ How it works

Under the hood, `tunfetch` compiles a Rust networking stack to WebAssembly, giving you native-level performance inside the Workers runtime. The WASM module handles the full TCP + TLS + proxy handshake, so you get SOCKS5 and HTTP CONNECT support that isn't available via the standard Workers `fetch()`.

## 📄 License

[LGPL-3.0](./LICENSE)