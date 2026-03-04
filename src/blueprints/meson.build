blueprints = custom_target('blueprints',
    input: files(
        'window.blp',
        'get-started-dialog.blp',
        'not-found-page.blp',
        'rom.blp',
        'roms-page.blp',
        'exception-dialog.blp',
        'upload-alert.blp'
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