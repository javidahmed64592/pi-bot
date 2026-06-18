Software Maintenance Guide
===========================

This document outlines how to configure and setup a development environment to work on |project_name|.

Backend (Python)
----------------

.. image:: https://img.shields.io/badge/Python-3.13-3776AB?style=flat-square&logo=python&logoColor=ffd343
   :target: https://docs.python.org/3.13/
   :alt: Python

.. image:: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/astral-sh/uv/main/assets/badge/v0.json&style=flat-square
   :target: https://docs.astral.sh/uv/
   :alt: uv

.. image:: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/astral-sh/ruff/main/assets/badge/v2.json&style=flat-square
   :target: https://docs.astral.sh/ruff/
   :alt: Ruff

.. image:: https://img.shields.io/badge/ty-Latest-2A6DB2?style=flat-square
   :target: https://docs.astral.sh/ty/
   :alt: Ty

.. image:: https://img.shields.io/badge/Sphinx-Latest-000000?style=flat-square&logo=sphinx&logoColor=white
   :target: https://www.sphinx-doc.org/
   :alt: Sphinx

Installing Dependencies
~~~~~~~~~~~~~~~~~~~~~~~

This repository is managed using the ``uv`` Python project manager: https://docs.astral.sh/uv/

Install the required dependencies:

.. code-block:: sh

   uv sync

To include extra dependencies:

.. code-block:: sh

   uv sync --extra dev
   uv sync --extra docs
   uv sync --all-extras

Testing, Linting, and Type Checking
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

.. code-block:: sh

   # Validate pyproject.toml
   uv run validate-pyproject pyproject.toml

   # Lint code
   uv run ruff check .

   # Format code
   uv run ruff format .

   # Type check
   uv run ty check .

   # Run tests
   uv run pytest

   # Security scan
   uv run bandit -r |package_name|/

   # Audit dependencies
   uv run pip-audit

Building Documentation
~~~~~~~~~~~~~~~~~~~~~~

This project uses Sphinx for documentation. To build the documentation:

.. code-block:: sh

   uv run sphinx-build -M clean docs/source/ docs/build/
   uv run sphinx-build -M html docs/source/ docs/build/

The built documentation will be available at ``docs/build/html/index.html``.
