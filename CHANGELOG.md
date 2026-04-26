# Changelog

## [0.2.0](https://github.com/codercodingthecode/sun-burn/compare/v0.1.0...v0.2.0) (2026-04-26)


### Features

* list_ssh_keys command — auto-detect ~/.ssh/*.pub files in SSH picker ([730cb70](https://github.com/codercodingthecode/sun-burn/commit/730cb70c0e5c3ed9d687a1ea1817fb4c8417428a))
* log progress every 5%, bump flash buffer to 16MB ([32ec86f](https://github.com/codercodingthecode/sun-burn/commit/32ec86fd7677d56523972f4f8a1e5a5127866bcf))


### Bug Fixes

* attach-release permissions, wire real file dialog and remove mock fallbacks ([85ef7fa](https://github.com/codercodingthecode/sun-burn/commit/85ef7fa3644d8c9d5a59f661edd4b8e72093b3ce))
* flash uses temp script file + progress poll file (avoid AppleScript escaping) ([1e4879f](https://github.com/codercodingthecode/sun-burn/commit/1e4879fedaea497084e232e8846cdd4c7a0323c4))
* live CoreWLAN WiFi scan via wifi_scan crate, geolocation prompt for permission ([dc29a60](https://github.com/codercodingthecode/sun-burn/commit/dc29a6048a7ff667f7b57c8678c1c90ff75b7f28))
* password show/hide toggle, SSH selected hover state, flash via osascript admin with dd progress ([6311794](https://github.com/codercodingthecode/sun-burn/commit/63117945d5c5c18b7010030b4ddb279450c262c0))
* real CoreWLAN scan with CLLocationManager permission via objc2 ([81bcf71](https://github.com/codercodingthecode/sun-burn/commit/81bcf71dc92e157e5ca6473935506892ed2f9292))
* remove redundant per-percent log entries (info already in progress ring) ([8ed9e47](https://github.com/codercodingthecode/sun-burn/commit/8ed9e4793bc17b79b1c24e03c3922e97ab766a1a))
* replace airport with system_profiler for WiFi scanning on macOS 15+ ([f3fdd10](https://github.com/codercodingthecode/sun-burn/commit/f3fdd10dbe260627930b27941112d43a61a96808))
* request Location permission for WiFi scanning on macOS ([4780de5](https://github.com/codercodingthecode/sun-burn/commit/4780de5cb5946797798190b771d48e58f20aa3c3))
* resetWizard explicitly clears each field so 'Flash another' works ([1d53059](https://github.com/codercodingthecode/sun-burn/commit/1d5305982ef133a0441d9fb8b5b750becf9a85db))
* stage image to /tmp before flash to bypass macOS TCC restriction on Downloads ([0bc6e82](https://github.com/codercodingthecode/sun-burn/commit/0bc6e829dde2c4520bd8b2379a076a89709c6334))
* use authopen instead of osascript+dd — bypasses TCC restriction on raw disk ([e4b8985](https://github.com/codercodingthecode/sun-burn/commit/e4b89856fe3d836fa3e63f52332aa8dfbcfa0229))
* use wifi_scan crate (CoreWLAN) for macOS WiFi scanning, no Location permission needed ([9a2de6f](https://github.com/codercodingthecode/sun-burn/commit/9a2de6f5e35abf9f9c22e0faa7eb81f1d9df0686))
* wifi-picker text input fallback, filter redacted SSIDs ([99a6935](https://github.com/codercodingthecode/sun-burn/commit/99a69359d2ec6f539d289a3d2c005b4ee9721810))
* **wifi:** use networksetup preferred-networks on macOS 15+ ([71226ff](https://github.com/codercodingthecode/sun-burn/commit/71226fff33473f5d38999a0a26659b66be689227))
* wire real flash and patch commands, remove all mock fallbacks ([8848681](https://github.com/codercodingthecode/sun-burn/commit/8848681d9cbbc9acedf1ad7b6c7590c6ee1d0978))
