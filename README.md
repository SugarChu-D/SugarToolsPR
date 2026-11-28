気が向いたら追記します。

## コード品質の基準

PRと `main` へのpushでは、Windows上のRust stableで次の3項目を確認します。ローカルでもリポジトリのルートから実行できます。

```sh
cargo +stable fmt --all -- --check
cargo +stable clippy --workspace --all-targets -- -D warnings
cargo +stable test --workspace
```

- Clippyの指摘を意図的に残す場合は、対象の関数に `#[expect(clippy::対象のlint, reason = "理由")]` を付けます。指摘が解消されると、不要になった例外もCIで検出できます。
- 乱数処理を変更するときは、既知のseedから得られる値と乱数を消費する順序を確認します。
- GPU実機が必要なテストや長時間検索は `#[ignore]` で個別に実行します。たとえばCPUとGPUの照合は `cargo test -p rng-core test_sha1_mt_matches_cpu_single_case -- --ignored` で確認できます。

## DSプロファイル

`config.toml` の `[ds_configs.プロファイル名]` にDS設定を書きます。プロファイルが1件の場合、CLIはその設定を自動で使います。複数ある場合は名前順の一覧から使用するプロファイルの番号を入力してください。

## GPU tuning

`config.toml` のトップレベルに `workgroup_size` を置くと、GPU ごとに compute workgroup size を指定できます。未指定時は `256` です。

```toml
workgroup_size = 64

[ds_configs.profile1]
version = "White2"
region = "JPN"
timer0 = 0x10fa
is_dslite = false
mac = 0x0009bf6d93ce
```

まずは `64`、`128`、`256` を比較してください。値は実行時に shader pipeline へ適用されるため、値ごとに別の実行ファイルをビルドする必要はありません。

## リリースの仕方

`cargo build -p [アプリ名] --release`でreleaseフォルダにexeファイルが生成される。
