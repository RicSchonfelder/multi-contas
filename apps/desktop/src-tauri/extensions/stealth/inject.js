;(function () {
  'use strict'

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

  function defGetter(proto, key, fn) {
    try {
      Object.defineProperty(proto, key, {
        get: fn,
        configurable: false,
        enumerable: true,
      })
    } catch (_) {}
  }

  defGetter(Navigator.prototype, 'webdriver', () => false)
  defGetter(Navigator.prototype, 'userAgent', () => cfg.userAgent)
  defGetter(Navigator.prototype, 'platform', () => cfg.platform)
  defGetter(Navigator.prototype, 'language', () => cfg.language)
  defGetter(Navigator.prototype, 'languages', () => [...cfg.languages])
  defGetter(
    Navigator.prototype,
    'hardwareConcurrency',
    () => cfg.hardwareConcurrency,
  )
  defGetter(Navigator.prototype, 'deviceMemory', () => cfg.deviceMemory)
  defGetter(Navigator.prototype, 'maxTouchPoints', () => cfg.maxTouchPoints)

  try {
    if (window.chrome && chrome.runtime) {
      defGetter(chrome.runtime, 'id', () => undefined)
    }
  } catch (_) {}
})()
