project(
  'imyemail-cloud',
  version: '0.2.2',
  meson_version: '>= 1.3.0',
  license: 'MIT',
)

i18n = import('i18n')
gnome = import('gnome')

prefix = get_option('prefix')
bindir = prefix / get_option('bindir')
datadir = prefix / get_option('datadir')
application_id = 'email.imy.cloud'

cargo = find_program('cargo')
cargo_script = find_program('build-aux/cargo.sh')

custom_target(
  'imyemail-cloud',
  build_by_default: true,
  output: 'imyemail-cloud',
  console: true,
  install: true,
  install_dir: bindir,
  command: [
    cargo_script,
    meson.project_build_root(),
    meson.project_source_root(),
    '@OUTPUT@',
    get_option('buildtype'),
  ],
)

install_data(
  'data/email.imy.cloud.desktop',
  install_dir: datadir / 'applications',
)
install_data(
  'data/email.imy.cloud.metainfo.xml',
  install_dir: datadir / 'metainfo',
)
install_data(
  'data/icons/email.imy.cloud.svg',
  install_dir: datadir / 'icons/hicolor/scalable/apps',
)
install_data(
  'data/icons/email.imy.cloud-symbolic.svg',
  install_dir: datadir / 'icons/hicolor/symbolic/apps',
)
foreach size : ['16', '24', '32', '48', '64', '128', '256', '512']
  install_data(
    'data/icons/hicolor' / (size + 'x' + size) / 'apps/email.imy.cloud.png',
    install_dir: datadir / 'icons/hicolor' / (size + 'x' + size) / 'apps',
  )
endforeach
subdir('po')
