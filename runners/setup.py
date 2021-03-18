from setuptools import setup, find_packages

setup(name='collab-fuzz-runner',
      version='0.2',
      packages=find_packages(),
      entry_points={
          'console_scripts': ['collab_fuzz_compose = runner.__main__:main',
                              'collab_fuzz_run = runner.runqueue:main',
                              'collab_fuzz_build = runner.build:main',
                              'collab_fuzz_plot = runner.plot:main'
                              ]
      },
      install_requires=['docker>=4.1', 'pyyaml>=5.0'])
