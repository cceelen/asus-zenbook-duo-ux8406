# Changelog

## [0.5.0](https://github.com/cceelen/asus-zenbook-duo-ux8406/compare/v0.4.0...v0.5.0) (2026-10-07)


### Features

* **release:** put the provenance bundle of the source archive on the release ([5985663](https://github.com/cceelen/asus-zenbook-duo-ux8406/commit/59856638aeadb05264ddf2683a5683ba86ab7893))


### Bug Fixes

* **release:** clone aports from upstream and push to the fork ([8817951](https://github.com/cceelen/asus-zenbook-duo-ux8406/commit/8817951c11243328dc49fc333255d910a32e1f1e))
* **release:** retry the aports pushes with back-off and print the merge request link when glab fails ([3a69b0b](https://github.com/cceelen/asus-zenbook-duo-ux8406/commit/3a69b0b39329659a54c3f9d50ebb6b0822000e91))
* **release:** run the aports job in an Alpine container ([3bc10db](https://github.com/cceelen/asus-zenbook-duo-ux8406/commit/3bc10db11fe9d2c20714acf5f9d70786b8ff3924))
* **release:** set the version in fuzz/Cargo.lock too ([096d45c](https://github.com/cceelen/asus-zenbook-duo-ux8406/commit/096d45cfdd56d3ae28979785d18074ea9c1aad4b))
* **release:** use SSH for aports and open the merge request with glab ([3a4618c](https://github.com/cceelen/asus-zenbook-duo-ux8406/commit/3a4618cc707df33cbcdcaeab5df16e5747f92d27))
* **rotation:** show Auto-rotate only where the extension can turn a panel ([f981325](https://github.com/cceelen/asus-zenbook-duo-ux8406/commit/f981325cb1bbce04665d2b2bb43d4d2f2c80b6b7)), closes [#25](https://github.com/cceelen/asus-zenbook-duo-ux8406/issues/25)
* **rotation:** turn the screens by hand where no sensor is found ([9fb3d5e](https://github.com/cceelen/asus-zenbook-duo-ux8406/commit/9fb3d5e6f1ba78db5dc0ffca8d92f6a30bf94518))

## [0.4.0](https://github.com/cceelen/asus-zenbook-duo-ux8406/compare/v0.3.0...v0.4.0) (2026-10-07)


### Features

* **release:** publish Alpine and Nix packages with attested SBOMs ([4ea5082](https://github.com/cceelen/asus-zenbook-duo-ux8406/commit/4ea5082e444d369b9a2a1dc5212ceae8be6653ac)), closes [#35](https://github.com/cceelen/asus-zenbook-duo-ux8406/issues/35)


### Bug Fixes

* **nix:** keep the build tools out of the keyboard package's closure ([8ddefb6](https://github.com/cceelen/asus-zenbook-duo-ux8406/commit/8ddefb662d0d251634bee58abe8e60b8bd53eb1d))

## [0.3.0](https://github.com/cceelen/asus-zenbook-duo-ux8406/compare/v0.2.1...v0.3.0) (2026-10-07)


### Features

* **keyboard-bpf:** keep the light and the row mode between connections ([3afaebb](https://github.com/cceelen/asus-zenbook-duo-ux8406/commit/3afaebb760f71730aed7471381d487fefe681238))


### Bug Fixes

* **release:** set the version of each crate of the tree in Cargo.lock ([e2796e8](https://github.com/cceelen/asus-zenbook-duo-ux8406/commit/e2796e8f1e2eda657eb90ae5b3402b72ecd5dfdd))

## [0.2.1](https://github.com/cceelen/asus-zenbook-duo-ux8406/compare/v0.2.0...v0.2.1) (2026-10-07)


### Bug Fixes

* **gnome-keys:** give each touchscreen its panel ([c81baa3](https://github.com/cceelen/asus-zenbook-duo-ux8406/commit/c81baa3521c66c5b45c854195c371131e5bf6768))

## [0.2.0](https://github.com/cceelen/asus-zenbook-duo-ux8406/compare/v0.1.3...v0.2.0) (2026-10-07)


### Features

* **rotation:** add an extension that turns the built-in screens ([757975f](https://github.com/cceelen/asus-zenbook-duo-ux8406/commit/757975f89b13b3d7f2208e9fd4cae8888b926848))


### Bug Fixes

* **gnome-keys:** keep the layout sound when the lower panel is switched ([89437ad](https://github.com/cceelen/asus-zenbook-duo-ux8406/commit/89437ad3af0b7c9c133a1cdd411466cefb999b3e))
* **gnome:** keep underscanning and the monitors for lease in a new layout ([8504cb2](https://github.com/cceelen/asus-zenbook-duo-ux8406/commit/8504cb21f492effdab7d4b59b673ec31782d4894))
* **rotation:** correct the sensor claim and the behaviour at a screen lock ([fc44470](https://github.com/cceelen/asus-zenbook-duo-ux8406/commit/fc4447050e1535763fe64acf21a25a008b3cd3d8))

## [0.1.3](https://github.com/cceelen/asus-zenbook-duo-ux8406/compare/v0.1.2...v0.1.3) (2026-10-06)


### Bug Fixes

* **debian13:** crate ignore 0.4.30 has issues on debian 13 ([2fe788c](https://github.com/cceelen/asus-zenbook-duo-ux8406/commit/2fe788c896e7561912137b6e93171823eb001456))
* **packaging:** start the guard service on installation ([0f48040](https://github.com/cceelen/asus-zenbook-duo-ux8406/commit/0f480406755a31fc5cf103e041f93ce3e4329a26))
* **packit:** build for RHEL 10 with EPEL 10 ([56b6118](https://github.com/cceelen/asus-zenbook-duo-ux8406/commit/56b61188e8f2a8de070d6c83008d98cba36744cd))

## [0.1.2](https://github.com/cceelen/asus-zenbook-duo-ux8406/compare/v0.1.1...v0.1.2) (2026-10-06)


### Bug Fixes

* **release:** find the release pull request without a search ([af8601c](https://github.com/cceelen/asus-zenbook-duo-ux8406/commit/af8601c61f498fd15d44c72b01277589d06e6665))
* **release:** make releases with Release Please ([8dac6ee](https://github.com/cceelen/asus-zenbook-duo-ux8406/commit/8dac6ee391a75d6f9b8f1e591c0b73690f5e2ca9))
