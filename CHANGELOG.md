# Changelog

All notable changes to this project will be documented in this file.

## [0.15.0] - 2025-10-18

[38a4ff1](38a4ff1b06f396a501bee1990a008476560ae879)...[466ab8a](466ab8a182c67225d0b67679552e29d810467dae)

### 🚀 Features

- Add copy button to diff page for command parameters (#132) ([63b2d26](https://github.com/MooncellWiki/ak-asset-storage/commit/63b2d26446799e9ba4d5581e0cab38677ff4bca8)), Co-authored-by:daflyinbed <21363956+daflyinbed@users.noreply.github.com>

---------, Co-authored-by:copilot-swe-agent[bot] <198982749+Copilot@users.noreply.github.com>, Co-authored-by:daflyinbed <21363956+daflyinbed@users.noreply.github.com>
- Implement GitHub-style dual-panel asset viewer (#134) ([466ab8a](https://github.com/MooncellWiki/ak-asset-storage/commit/466ab8a182c67225d0b67679552e29d810467dae)), Co-authored-by:daflyinbed <21363956+daflyinbed@users.noreply.github.com>

* fix: add error handling for API failures in AssetTree, Co-authored-by:daflyinbed <21363956+daflyinbed@users.noreply.github.com>

* fix: rename catch parameter to follow linting rules, Co-authored-by:daflyinbed <21363956+daflyinbed@users.noreply.github.com>

* refactor: address code review feedback

- Fix children initialization to undefined for NTree lazy loading
- Move breadcrumb navigation to index.vue header
- Replace custom resizer with NSplit component
- Use useRouteQuery for URL synchronization
- Lift data management to index.vue to share between tree and content
- Fix icon centering in breadcrumb
- Move mobile menu button next to breadcrumb, Co-authored-by:daflyinbed <21363956+daflyinbed@users.noreply.github.com>

* refactor: improve data flow and type consistency

- Create shared TreeNode type in app/pages/asset/types/index.ts
- Change AssetTree to receive onLoad as prop instead of emitting
- Pass file size from treeData to AssetContent via props
- Remove unnecessary API call in AssetContent for file size
- Add size field to TreeNode interface for better data passing, Co-authored-by:daflyinbed <21363956+daflyinbed@users.noreply.github.com>

* fix: improve tree navigation and async handling

- Remove expand-on-click from NTree to prevent unwanted expansions
- Change onLoad prop to always return Promise<void> for consistency
- Add manual onLoad handling in watch for selectedPath
- Ensure tree expands and loads necessary nodes when path changes
- Await onLoad calls in handleSearchSelect for proper loading, Co-authored-by:daflyinbed <21363956+daflyinbed@users.noreply.github.com>

* refactor

* feat: apt path from env

---------, Co-authored-by:copilot-swe-agent[bot] <198982749+Copilot@users.noreply.github.com>, Co-authored-by:daflyinbed <21363956+daflyinbed@users.noreply.github.com>, Co-authored-by:xwbx <1677759063@qq.com>

### 📚 Documentation

- 兼容copilot ([e460918](https://github.com/MooncellWiki/ak-asset-storage/commit/e460918a24f973b361e0f86c327ff7bde4f5bae6))

## [0.14.0] - 2025-09-23

[715d7cf](715d7cfe2600fe26144cf7c7a281b7c53fe0df33)...[38a4ff1](38a4ff1b06f396a501bee1990a008476560ae879)

### 🐛 Bug Fixes

- Clippy ([e70d3f7](https://github.com/MooncellWiki/ak-asset-storage/commit/e70d3f7c0cc3da51e891348e8d270de503f5f79d))

### ⚙️ Miscellaneous Tasks

- Update dependencies ([90b2ae9](https://github.com/MooncellWiki/ak-asset-storage/commit/90b2ae97c18c760e58cb34ccb88b8ee7c1855983))
- Release 0.14.0 ([38a4ff1](https://github.com/MooncellWiki/ak-asset-storage/commit/38a4ff1b06f396a501bee1990a008476560ae879))

## [0.13.3] - 2025-08-15

[0bfddb6](0bfddb6a45de8b91b11e61837cc7d19c2a37f2ae)...[715d7cf](715d7cfe2600fe26144cf7c7a281b7c53fe0df33)

### 🐛 Bug Fixes

- Memory leak ([39c4d48](https://github.com/MooncellWiki/ak-asset-storage/commit/39c4d4807c75bd4dc1dfe6deb0205e3e43bc0db5))

### ⚙️ Miscellaneous Tasks

- Release 0.13.3 ([715d7cf](https://github.com/MooncellWiki/ak-asset-storage/commit/715d7cfe2600fe26144cf7c7a281b7c53fe0df33))

## [0.13.2] - 2025-08-15

[d73282e](d73282e44b9556594a3870659ab584fd5ee661ec)...[0bfddb6](0bfddb6a45de8b91b11e61837cc7d19c2a37f2ae)

### 🐛 Bug Fixes

- Memory leak ([c5f07cf](https://github.com/MooncellWiki/ak-asset-storage/commit/c5f07cf85d3037e2370e59bd88f595dd0b366821))

### ⚙️ Miscellaneous Tasks

- Fix lint-stage ([6c342dc](https://github.com/MooncellWiki/ak-asset-storage/commit/6c342dce682053302c2bd8170612f78bb4ae7c40))
- Release 0.13.2 ([0bfddb6](https://github.com/MooncellWiki/ak-asset-storage/commit/0bfddb6a45de8b91b11e61837cc7d19c2a37f2ae))

## [0.13.1] - 2025-08-15

[19b0e76](19b0e769df22eb5f6b557fdd05440f5b0493d262)...[d73282e](d73282e44b9556594a3870659ab584fd5ee661ec)

### 🚀 Features

- Docker support attach to network ([1adb98e](https://github.com/MooncellWiki/ak-asset-storage/commit/1adb98ee105086e38561f4f98468627ee07e4248))

### ⚙️ Miscellaneous Tasks

- Enable rust cache ([5c78fde](https://github.com/MooncellWiki/ak-asset-storage/commit/5c78fde008298f12db4de89754c0d2b22eeafe64))
- Run cargo fmt and cargo clippy when has rs file change ([1fc921a](https://github.com/MooncellWiki/ak-asset-storage/commit/1fc921ab316fe00da1194f4b0066513475b40d7c))
- Release 0.13.1 ([d73282e](https://github.com/MooncellWiki/ak-asset-storage/commit/d73282e44b9556594a3870659ab584fd5ee661ec))

## [0.13.0] - 2025-08-15

[7391e1b](7391e1b4b20db3656dc46d4a90fd333d9bcec294)...[19b0e76](19b0e769df22eb5f6b557fdd05440f5b0493d262)

### 🚀 Features

- Docker api ([7ba9be4](https://github.com/MooncellWiki/ak-asset-storage/commit/7ba9be4994e784daf67bb86316a027b163d90574))

### 🐛 Bug Fixes

- Layout (#128) ([9ffb981](https://github.com/MooncellWiki/ak-asset-storage/commit/9ffb9812b39e7d13560cd1c4d851eda0fe77b819)), Co-authored-by:StarHeartHunt <starheart233@gmail.com>

### ⚡ Performance

- Use mimalloc ([2c3c761](https://github.com/MooncellWiki/ak-asset-storage/commit/2c3c76115dda19ad01b6cfd5d9664a4bf0670ebc))

### ⚙️ Miscellaneous Tasks

- Release 0.13.0 ([19b0e76](https://github.com/MooncellWiki/ak-asset-storage/commit/19b0e769df22eb5f6b557fdd05440f5b0493d262))

## [0.12.2] - 2025-08-04

[a86eb3c](a86eb3cd8cbfbdf0896c4f60199e9a1954f994ff)...[7391e1b](7391e1b4b20db3656dc46d4a90fd333d9bcec294)

### 🐛 Bug Fixes

- Start download immedate ([4bb7d21](https://github.com/MooncellWiki/ak-asset-storage/commit/4bb7d21847d8e309157ce501e8e936e5880ca0bf))

### ⚙️ Miscellaneous Tasks

- Release 0.12.2 ([7391e1b](https://github.com/MooncellWiki/ak-asset-storage/commit/7391e1b4b20db3656dc46d4a90fd333d9bcec294))

## [0.12.1] - 2025-08-02

[13a007e](13a007eab57e6018d615df201462957f3228836a)...[a86eb3c](a86eb3cd8cbfbdf0896c4f60199e9a1954f994ff)

### 🐛 Bug Fixes

- Pull docker image with credentials ([dc1d79e](https://github.com/MooncellWiki/ak-asset-storage/commit/dc1d79e9d0d2a76e497c382bc208a490b4ef83d1))
- Sentry ([a7a3eb8](https://github.com/MooncellWiki/ak-asset-storage/commit/a7a3eb86e319424763515e9c21a67a2fb8d66cf8))
- Download ([472740e](https://github.com/MooncellWiki/ak-asset-storage/commit/472740ef80accd92433e18a1c497fcd5f39297c1))

### ⚙️ Miscellaneous Tasks

- Add claude-code ([e824720](https://github.com/MooncellWiki/ak-asset-storage/commit/e824720ead3adbb76d4fc23b4eb3035353ac1f8e))
- Fix claude-code ([ee76124](https://github.com/MooncellWiki/ak-asset-storage/commit/ee761241bb51744fad9e48d1fc39d7cccd0aa48f))
- Release 0.12.1 ([a86eb3c](https://github.com/MooncellWiki/ak-asset-storage/commit/a86eb3cd8cbfbdf0896c4f60199e9a1954f994ff))

## [0.12.0] - 2025-07-31

[eede223](eede2231e9098e6a1161cfb1fc79b92eb7e6ad18)...[13a007e](13a007eab57e6018d615df201462957f3228836a)

### 🚀 Features

- Support like search ([c4f7060](https://github.com/MooncellWiki/ak-asset-storage/commit/c4f70602b715b094ffc73867a39211e9589a32c0))
- 显示完整路径 ([8374a6f](https://github.com/MooncellWiki/ak-asset-storage/commit/8374a6fb356e5af2cf00ce8d8d23d025ae44b87f))

### 🐛 Bug Fixes

- File table ([c5397d6](https://github.com/MooncellWiki/ak-asset-storage/commit/c5397d647a45154896f4450f30c7d1273d053276))

### ⚙️ Miscellaneous Tasks

- Add contributing.md ([e2e7caf](https://github.com/MooncellWiki/ak-asset-storage/commit/e2e7caf1b3b78e2b10ef443f75ad3fc9c6fcbd37))
- Update dependencies ([112fb6b](https://github.com/MooncellWiki/ak-asset-storage/commit/112fb6b29335cf107a00c86385ee3e13a18c1639))
- Update rust ([aa24b98](https://github.com/MooncellWiki/ak-asset-storage/commit/aa24b9896bcf36f2102f2b9f07798a7df02bfc1d))
- Release 0.12.0 ([13a007e](https://github.com/MooncellWiki/ak-asset-storage/commit/13a007eab57e6018d615df201462957f3228836a))

## [0.11.0] - 2025-07-27

[03db722](03db722ca6a7ce7e8e1abea09fd3c97a1b1c7098)...[eede223](eede2231e9098e6a1161cfb1fc79b92eb7e6ad18)

### 🚀 Features

- Seed ([2169669](https://github.com/MooncellWiki/ak-asset-storage/commit/2169669146b9ee4174568aa53738f6bf8f861c1b))
- Openapi and scalar ([02f4417](https://github.com/MooncellWiki/ak-asset-storage/commit/02f4417a5f61fdfcb5b04e45c6a8475633fef480))
- Support minio ([2ad67cc](https://github.com/MooncellWiki/ak-asset-storage/commit/2ad67cc60b70942eeb9b28587a51ed626908c086)), feat:always download data from remote
- Serve frontend ([f66a881](https://github.com/MooncellWiki/ak-asset-storage/commit/f66a88125702ef185ef0eb1f6575013884194893))
- 显示版本状态 ([8ce2488](https://github.com/MooncellWiki/ak-asset-storage/commit/8ce24889ae12a0ed3f7207b56e31873eeb007de8))
- Support start server without start worker ([a9c2942](https://github.com/MooncellWiki/ak-asset-storage/commit/a9c294258f0a75e7c33965ee0a0ae97844e69d64))
- Version list接口不响应hot update list ([0730653](https://github.com/MooncellWiki/ak-asset-storage/commit/0730653f8cb41d0dc1f1115bb5052e22ac66d342))
- 支持版本没选全的情况 ([5dedd31](https://github.com/MooncellWiki/ak-asset-storage/commit/5dedd31ce03268cd598ba93adab409f1ec3ae053))
- 支持搜索 ([592cba7](https://github.com/MooncellWiki/ak-asset-storage/commit/592cba747d96789c7ac1260fd6d80053dba8bb1d))
- Better download file name ([4186210](https://github.com/MooncellWiki/ak-asset-storage/commit/4186210282f6c1e2f28e3580a5124cd5e5bdd29d))
- Add sentry for backend ([2ff948a](https://github.com/MooncellWiki/ak-asset-storage/commit/2ff948a37d9a5024ce7c176c7c82bee67b56d4f2))
- Concurrent download ([c63b73c](https://github.com/MooncellWiki/ak-asset-storage/commit/c63b73c17343143142a6b62153a349d9f9a8c000))
- Item_demand ([b9be383](https://github.com/MooncellWiki/ak-asset-storage/commit/b9be383104912e57b77bf6244b2e3b4318965057))
- Torappu assets ([f9264d6](https://github.com/MooncellWiki/ak-asset-storage/commit/f9264d6dfc2574beec58f6e961f4e0a4c9aaec50))
- Trigger Torappu and Ptilopsis Bot (#117) ([a670787](https://github.com/MooncellWiki/ak-asset-storage/commit/a67078774956af206ef37e09958e7be68ae67d68))

### 🐛 Bug Fixes

- Ci ([ef378a1](https://github.com/MooncellWiki/ak-asset-storage/commit/ef378a132c8fff86943212a302f958ed7ed4289c))
- Format ([a6d57b5](https://github.com/MooncellWiki/ak-asset-storage/commit/a6d57b529f1ad0efd15a9bd562be27b00c319e30))
- Features and lint ([58e71bf](https://github.com/MooncellWiki/ak-asset-storage/commit/58e71bf5e50da35208a433822fd1b782c90d13a6))
- Update title ([d54d4db](https://github.com/MooncellWiki/ak-asset-storage/commit/d54d4db33a2fcc58e2d17f0ab346265d97eeaf52))
- 优化seed逻辑 ([9e15d3c](https://github.com/MooncellWiki/ak-asset-storage/commit/9e15d3c74e9b33182cacfda41587d68d4d571a7d))
- Check resp code when download ([b57433c](https://github.com/MooncellWiki/ak-asset-storage/commit/b57433c0991891c692a33b24e6d6898ed2198c58))
- Fallback to index.html when asset not found ([5aabd20](https://github.com/MooncellWiki/ak-asset-storage/commit/5aabd20468d6c1063c600132eed04a274beeb871))
- Email url ([4214598](https://github.com/MooncellWiki/ak-asset-storage/commit/4214598955c67ff62985b96164180da94e377467))
- Update field ([b54e9bb](https://github.com/MooncellWiki/ak-asset-storage/commit/b54e9bbcdeba21feef1658ab7fb7f56247e4de41))
- Version list接口不响应hot update list ([dd9f08f](https://github.com/MooncellWiki/ak-asset-storage/commit/dd9f08ff5a1989f45eff266dcd8bd9bd881863dc))
- Content type ([09300c1](https://github.com/MooncellWiki/ak-asset-storage/commit/09300c18282dee7090e949ba4bdb64a590188533))
- 点到父节点上时不显示弹窗 ([9ffb3c5](https://github.com/MooncellWiki/ak-asset-storage/commit/9ffb3c5e3b2bfd00aa96f0a1c17156101f2862ae))
- Embed path ([8399e17](https://github.com/MooncellWiki/ak-asset-storage/commit/8399e170ed4af850e66281bbaebffcb1ee7743a2))
- Hash ([0bcc5dd](https://github.com/MooncellWiki/ak-asset-storage/commit/0bcc5dd1b515dbaa7b38f0ef30ca08fffddbad90))
- Tests ([7ca8938](https://github.com/MooncellWiki/ak-asset-storage/commit/7ca89388dc936f37061b696a5051b83dadefc8a6))
- Sqlx ([87e5063](https://github.com/MooncellWiki/ak-asset-storage/commit/87e5063e5ea3cc47c4e8ba8877514f97b8d2255d))
- Cargo release ([22b9e69](https://github.com/MooncellWiki/ak-asset-storage/commit/22b9e6938f83ae79f42ac20af6b8ab5a9433e78b))
- Dockerfile ([32e7166](https://github.com/MooncellWiki/ak-asset-storage/commit/32e7166f2aa8713fedb522e88d9724fde7bf4460))
- Dockerfile ([ddb23bd](https://github.com/MooncellWiki/ak-asset-storage/commit/ddb23bdc6aa6de3a7d9cb6bc1fa85edf3282a915))
- Align path with torappu backend ([3c4976f](https://github.com/MooncellWiki/ak-asset-storage/commit/3c4976faf1a2895611646afc1b56dbfd7189caba))
- Front end asset dir ([a51ce33](https://github.com/MooncellWiki/ak-asset-storage/commit/a51ce33abde09ff21a951c83ceec8c81b44b44ed))

### 🚜 Refactor

- Clippy ([270fdb6](https://github.com/MooncellWiki/ak-asset-storage/commit/270fdb6318889626f5d729a49b44c6e3ee513da9))
- Split download and check task ([d404f97](https://github.com/MooncellWiki/ak-asset-storage/commit/d404f975ee26ad3096c775131b04273f400a85db))
- Remove loco ([b474219](https://github.com/MooncellWiki/ak-asset-storage/commit/b4742195b3b77137810034dd587d348faeb04ea2))
- Redesign database layout ([e074a82](https://github.com/MooncellWiki/ak-asset-storage/commit/e074a821af1cbb2b51d479c7fb125795b753f9f7))
- Monorepo ([80e7ca7](https://github.com/MooncellWiki/ak-asset-storage/commit/80e7ca75554efe2deea963ea8c6b8ff67a2db751))
- Move position ([15bca67](https://github.com/MooncellWiki/ak-asset-storage/commit/15bca67683dc1b776824ca50745d5f19729280c1))
- 拉平目录结构，从sea-orm换成sqlx ([1bcf9d9](https://github.com/MooncellWiki/ak-asset-storage/commit/1bcf9d9e94b04d67c3a941690d72b8e3e50fc232))

### 📚 Documentation

- Update readme ([db0a5c9](https://github.com/MooncellWiki/ak-asset-storage/commit/db0a5c92c7ae588c3f364887cbf4c26b2f145823)), chore:format
- Update sql ([87cc180](https://github.com/MooncellWiki/ak-asset-storage/commit/87cc180cba44d19f9fbca0bbda7f9b81aa0efcfb))

### ⚙️ Miscellaneous Tasks

- Update git hook ([129f669](https://github.com/MooncellWiki/ak-asset-storage/commit/129f66998cce863ec01c7c40d607647e46cf8996))
- Update gitignore ([734d649](https://github.com/MooncellWiki/ak-asset-storage/commit/734d649873ce183498e775fdf634ebc892b42c0d))
- Release tools ([50dc5c2](https://github.com/MooncellWiki/ak-asset-storage/commit/50dc5c2c6011dfc6c90200d620d520c060f06b32))
- Fix ci ([ede4baf](https://github.com/MooncellWiki/ak-asset-storage/commit/ede4baf6839964a5e50583b84b37cafa6024a8af))
- Setup git-cliff and cargo release ([095113a](https://github.com/MooncellWiki/ak-asset-storage/commit/095113a95980a53c13720afe8d295f8ddd5ef7f7))
- Release 0.1.0 ([2c595c2](https://github.com/MooncellWiki/ak-asset-storage/commit/2c595c2a8fd0dd9e77a97ec196e04bf7c2ec9b3a))
- Run eslint in ci ([228d265](https://github.com/MooncellWiki/ak-asset-storage/commit/228d265dbcb876bcca401e098705ddfeeb88853a))
- Release docker image ([c5a6d4c](https://github.com/MooncellWiki/ak-asset-storage/commit/c5a6d4c54ef1595c4ccab98713ffb7066a013178))
- Fix eslint ([9042642](https://github.com/MooncellWiki/ak-asset-storage/commit/9042642909b310140fb175771c6475c6d45c62a3))
- Update changelog template ([799d1a0](https://github.com/MooncellWiki/ak-asset-storage/commit/799d1a0a2ff5638d0e4659c48d14d7b8dc148758))
- Release 0.2.0 ([8af205a](https://github.com/MooncellWiki/ak-asset-storage/commit/8af205a81d1c7a03ab32f85d10330832e83aaa27))
- Release 0.3.0 ([b979426](https://github.com/MooncellWiki/ak-asset-storage/commit/b979426e7d60c308fb149f385243f1bb0e99b6e9))
- Update dependencies ([48be2fc](https://github.com/MooncellWiki/ak-asset-storage/commit/48be2fcfbd38164746a16634e87df8cfe5123009))
- Release 0.4.0 ([364ef9c](https://github.com/MooncellWiki/ak-asset-storage/commit/364ef9c890a2a8536b25429921ad113d7cdcc39e))
- Update dependencies ([47fe5b7](https://github.com/MooncellWiki/ak-asset-storage/commit/47fe5b76e82e0f03e0116a25a6f499d08b5d70a1))
- Update dependencies ([6c099e5](https://github.com/MooncellWiki/ak-asset-storage/commit/6c099e58bfe0d9b54d3929a61812548b99c1f54e))
- Update pnpm to v9.15.0 (#27) ([453dee3](https://github.com/MooncellWiki/ak-asset-storage/commit/453dee3b42712559f45b0bc5ae2e1909aa6d7507)), Co-authored-by:renovate[bot] <29139614+renovate[bot]@users.noreply.github.com>
- Update tokio-tracing monorepo (#26) ([10a9c16](https://github.com/MooncellWiki/ak-asset-storage/commit/10a9c16cdcf3eb5296fc2dcfa7fb32487156b0ed)), Co-authored-by:renovate[bot] <29139614+renovate[bot]@users.noreply.github.com>
- Lock file maintenance (#28) ([6b9de3a](https://github.com/MooncellWiki/ak-asset-storage/commit/6b9de3a406179f71a99b47bf791afcd6e445a9e0)), Co-authored-by:renovate[bot] <29139614+renovate[bot]@users.noreply.github.com>
- Release 0.5.0 ([1a3d11f](https://github.com/MooncellWiki/ak-asset-storage/commit/1a3d11f7e01209757084f96fbb3d15c8beee5bdd))
- Release 0.5.1 ([0d32eb5](https://github.com/MooncellWiki/ak-asset-storage/commit/0d32eb583b6974448079f5e624071b067bc3e16a))
- Release 0.5.2 ([e816993](https://github.com/MooncellWiki/ak-asset-storage/commit/e816993999b50e6fb21904c0e1846bd8f67850c4))
- Release 0.6.0 ([0b1a038](https://github.com/MooncellWiki/ak-asset-storage/commit/0b1a038f74e7d037bac300875ec1cbe6c25a4d40))
- Release 0.7.0 ([88702fe](https://github.com/MooncellWiki/ak-asset-storage/commit/88702fef70939ace78bd2aafeb3101311824b4f8))
- Release 0.7.1 ([79324f6](https://github.com/MooncellWiki/ak-asset-storage/commit/79324f6747b7354abbb91cb17795147a65973845))
- Fix ([cd016c1](https://github.com/MooncellWiki/ak-asset-storage/commit/cd016c190f051b40a240529f9bc72cd3f227b22d))
- Update deps ([c8b23c2](https://github.com/MooncellWiki/ak-asset-storage/commit/c8b23c2ffb25ee40c349b697b5136a46ff863b65))
- Fix ([d2af6e3](https://github.com/MooncellWiki/ak-asset-storage/commit/d2af6e3e87a6e4f9c31cbd5466be12b861840f6b))
- Add desc ([7740ad0](https://github.com/MooncellWiki/ak-asset-storage/commit/7740ad07f35c3b73f1f5d6109c39df02f9a16879))
- Release 0.8.0 ([a4fc94f](https://github.com/MooncellWiki/ak-asset-storage/commit/a4fc94f850488becf53db5acfdc1e82bac27aee5))
- Disable debug sentry ([bcd81cd](https://github.com/MooncellWiki/ak-asset-storage/commit/bcd81cd4039bd7751686d26e0b0d2c706638ec6a))
- Release 0.8.1 ([ecf87e9](https://github.com/MooncellWiki/ak-asset-storage/commit/ecf87e9dcb37216a18036fb0388438464dcd6f42))
- Update ([a8bda4e](https://github.com/MooncellWiki/ak-asset-storage/commit/a8bda4e6aa86250a297f52191b8c9ba93d567b91))
- Update dependencies ([c8a2558](https://github.com/MooncellWiki/ak-asset-storage/commit/c8a2558481545f5c6c336e514d1acb3cac12f403))
- Update dependencies ([fbb79c8](https://github.com/MooncellWiki/ak-asset-storage/commit/fbb79c8327c89134f9d33eafcd52d265cdb19dc2))
- Update dependencies ([e0a1ea3](https://github.com/MooncellWiki/ak-asset-storage/commit/e0a1ea3870795c75710030f8d33845cdb84db85d))
- Update node ([366aaf4](https://github.com/MooncellWiki/ak-asset-storage/commit/366aaf4cdf61f45edc45ba556a7c59fc9f4fc721))
- Disable eslint ([5e76f49](https://github.com/MooncellWiki/ak-asset-storage/commit/5e76f490c316d43bd0d7f94fbf48d368deb84b8d))
- Release 0.9.0 ([dfbada0](https://github.com/MooncellWiki/ak-asset-storage/commit/dfbada0ab66b5f9b22247585541bf7eeaafe655c))
- Update dependencies ([316588b](https://github.com/MooncellWiki/ak-asset-storage/commit/316588b28aadc11c132e30ca9ad7c262bfed7d49))
- Release 0.10.0 ([f5ab6f7](https://github.com/MooncellWiki/ak-asset-storage/commit/f5ab6f701879d15f17060a01008dc880ef6e7e98))
- Release 0.10.1 ([674fb0f](https://github.com/MooncellWiki/ak-asset-storage/commit/674fb0f7b1ef1d2dcf0cc877cb0338553ac9d469))
- Release 0.10.2 ([25608e6](https://github.com/MooncellWiki/ak-asset-storage/commit/25608e65a0ade260c5a15c12f3fd0ddc5e6e6438))
- Release 0.10.3 ([9db419d](https://github.com/MooncellWiki/ak-asset-storage/commit/9db419dddd549965fd19375c6f117377b0f9b6a0))
- Release 0.10.4 ([87fd30b](https://github.com/MooncellWiki/ak-asset-storage/commit/87fd30bab1c3917876b29bf59f1e5b6c3f1d3d25))
- Fix changelog ([10f0d17](https://github.com/MooncellWiki/ak-asset-storage/commit/10f0d1739b428ff96f854a26367d2cc0d568b884))
- Release 0.11.0 ([eede223](https://github.com/MooncellWiki/ak-asset-storage/commit/eede2231e9098e6a1161cfb1fc79b92eb7e6ad18))

### Refactor

- Move to multi crates (#116) ([cdae1e3](https://github.com/MooncellWiki/ak-asset-storage/commit/cdae1e30c8ce14d1b34a22d44fa78c89b293a0c0))

<!-- generated by git-cliff -->
