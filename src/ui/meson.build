blueprints = custom_target('blueprints',
    input: files(
        'window.blp',
        'home_page.blp',
        'rom.blp',
        'roms_page.blp',
        'settings.blp'
    ),
    output: '.',
    command: [
      find_program('blueprint-compiler'),
      'batch-compile',
      '@OUTPUT@',
      '@CURRENT_SOURCE_DIR@',
      '@INPUT@'
    ]
)