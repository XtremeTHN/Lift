blueprints = custom_target('blueprints',
    input: files(
        'window.blp',
        'not-found-page.blp',
        'rom.blp',
        'roms-page.blp',
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