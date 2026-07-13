;(function () {
  'use strict'

  /* ── Default config (overridable via localStorage key 'mc_stealth_config') ── */
  const cfg = {
    userAgent:
      'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36',
    platform: 'Win32',
    language: 'pt-BR',
    languages: ['pt-BR', 'en-US', 'en'],
    hardwareConcurrency: 8,
    deviceMemory: 8,
    maxTouchPoints: 0,
    screenWidth: 1920,
    screenHeight: 1080,
    colorDepth: 24,
    pixelDepth: 24,
    webglVendor: 'Google Inc. (Intel)',
    webglRenderer:
      'ANGLE (Intel, Intel(R) UHD Graphics Direct3D11 vs_5_0 ps_5_0)',
    canvasNoise: true,
    audioNoise: true,
  }

  try {
    const stored = localStorage.getItem('mc_stealth_config')
    if (stored) Object.assign(cfg, JSON.parse(stored))
  } catch (_) {}

  /* ── Helpers ── */
  function defGetter(proto, key, fn) {
    try {
      Object.defineProperty(proto, key, {
        get: fn,
        configurable: false,
        enumerable: true,
      })
    } catch (_) {}
  }

  function defValue(proto, key, val) {
    try {
      Object.defineProperty(proto, key, {
        get: () => val,
        set: () => {},
        configurable: false,
        enumerable: true,
      })
    } catch (_) {}
  }

  /* ──────────────────────────────────────────────────
   * 1. Navigator
   * ────────────────────────────────────────────────── */
  const N = Navigator.prototype

  defGetter(N, 'webdriver', () => false)
  defGetter(N, 'userAgent', () => cfg.userAgent)
  defGetter(N, 'platform', () => cfg.platform)
  defGetter(N, 'language', () => cfg.language)
  defGetter(N, 'languages', () => [...cfg.languages])
  defGetter(N, 'hardwareConcurrency', () => cfg.hardwareConcurrency)
  defGetter(N, 'deviceMemory', () => cfg.deviceMemory)
  defGetter(N, 'maxTouchPoints', () => cfg.maxTouchPoints)

  const empty = {
    item: () => null,
    namedItem: () => null,
    refresh: () => {},
    length: 0,
    [Symbol.iterator]: function* () {},
    entries: () => [][Symbol.iterator](),
    keys: () => [][Symbol.iterator](),
    values: () => [][Symbol.iterator](),
    forEach: () => {},
  }
  defGetter(N, 'plugins', () => empty)
  defGetter(N, 'mimeTypes', () => empty)

  /* ──────────────────────────────────────────────────
   * 2. WebGL
   * ────────────────────────────────────────────────── */
  const VENDOR = 0x9245
  const RENDERER = 0x9246

  ;(function () {
    const p1 = WebGLRenderingContext.prototype
    const o1 = p1.getParameter
    p1.getParameter = function (p) {
      if (p === VENDOR) return cfg.webglVendor
      if (p === RENDERER) return cfg.webglRenderer
      return o1.call(this, p)
    }
    if (typeof WebGL2RenderingContext !== 'undefined') {
      const p2 = WebGL2RenderingContext.prototype
      const o2 = p2.getParameter
      p2.getParameter = function (p) {
        if (p === VENDOR) return cfg.webglVendor
        if (p === RENDERER) return cfg.webglRenderer
        return o2.call(this, p)
      }
    }
  })()

  /* ──────────────────────────────────────────────────
   * 3. Canvas
   * ────────────────────────────────────────────────── */
  if (cfg.canvasNoise) {
    ;(function () {
      const dataURL = HTMLCanvasElement.prototype.toDataURL
      HTMLCanvasElement.prototype.toDataURL = function (...a) {
        let c
        try {
          c = document.createElement('canvas')
          c.width = this.width
          c.height = this.height
          const ctx = c.getContext('2d', { willReadFrequently: true })
          if (ctx) {
            ctx.drawImage(this, 0, 0)
            const d = ctx.getImageData(0, 0, c.width, c.height)
            for (let i = 0; i < d.data.length; i += 64) d.data[i] ^= 1
            ctx.putImageData(d, 0, 0)
          }
        } catch (_) {
          c = this
        }
        return dataURL.apply(c, a)
      }

      const blob = HTMLCanvasElement.prototype.toBlob
      HTMLCanvasElement.prototype.toBlob = function (cb, ...a) {
        let c
        try {
          c = document.createElement('canvas')
          c.width = this.width
          c.height = this.height
          const ctx = c.getContext('2d', { willReadFrequently: true })
          if (ctx) {
            ctx.drawImage(this, 0, 0)
            const d = ctx.getImageData(0, 0, c.width, c.height)
            for (let i = 0; i < d.data.length; i += 64) d.data[i] ^= 1
            ctx.putImageData(d, 0, 0)
          }
        } catch (_) {
          c = this
        }
        return blob.call(c, cb, ...a)
      }

      const gID = CanvasRenderingContext2D.prototype.getImageData
      CanvasRenderingContext2D.prototype.getImageData = function (...a) {
        const img = gID.apply(this, a)
        const d = img.data
        for (let i = 0; i < d.length; i += 64) d[i] ^= 1
        return img
      }
    })()
  }

  /* ──────────────────────────────────────────────────
   * 4. Audio
   * ────────────────────────────────────────────────── */
  if (cfg.audioNoise) {
    ;(function () {
      const orig = AudioBuffer.prototype.getChannelData
      AudioBuffer.prototype.getChannelData = function (ch) {
        const buf = orig.call(this, ch)
        if (buf.length > 0) buf[0] *= 0.9999
        return buf
      }
    })()
  }

  /* ──────────────────────────────────────────────────
   * 5. Screen
   * ────────────────────────────────────────────────── */
  const S = Screen.prototype
  defValue(S, 'width', cfg.screenWidth)
  defValue(S, 'height', cfg.screenHeight)
  defValue(S, 'availWidth', cfg.screenWidth)
  defValue(S, 'availHeight', cfg.screenHeight)
  defValue(S, 'colorDepth', cfg.colorDepth)
  defValue(S, 'pixelDepth', cfg.pixelDepth)

  /* ──────────────────────────────────────────────────
   * 6. Hide chrome.runtime.id
   * ────────────────────────────────────────────────── */
  try {
    if (chrome && chrome.runtime) defGetter(chrome.runtime, 'id', () => undefined)
  } catch (_) {}

  /* ──────────────────────────────────────────────────
   * 7. WebRTC — strip host IPs from SDP
   * ────────────────────────────────────────────────── */
  try {
    const PC = window.RTCPeerConnection || window.webkitRTCPeerConnection
    if (!PC) throw 0

    const stripIPs = (sdp) =>
      sdp
        .replace(/a=candidate:\S+ \d+ \d+ \d+ (?:[0-9]{1,3}\.){3}[0-9]{1,3}/g, '')
        .replace(/c=IN IP4\s+\d+\.\d+\.\d+\.\d+/g, 'c=IN IP4 0.0.0.0')

    const oOffer = PC.prototype.createOffer
    PC.prototype.createOffer = function (...a) {
      return oOffer.apply(this, a).then((o) => {
        o.sdp = stripIPs(o.sdp)
        return o
      })
    }

    const oAnswer = PC.prototype.createAnswer
    PC.prototype.createAnswer = function (...a) {
      return oAnswer.apply(this, a).then((o) => {
        o.sdp = stripIPs(o.sdp)
        return o
      })
    }
  } catch (_) {}

  /* ──────────────────────────────────────────────────
   * 8. Inject main-world script (inject.js)
   * ────────────────────────────────────────────────── */
  function inject() {
    try {
      const s = document.createElement('script')
      s.src = chrome.runtime.getURL('inject.js')
      s.onload = () => s.remove()
      ;(document.head || document.documentElement || document).appendChild(s)
    } catch (_) {}
  }

  if (document.head || document.documentElement) {
    inject()
  } else {
    requestAnimationFrame(inject)
  }
})()
