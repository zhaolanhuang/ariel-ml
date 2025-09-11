Ariel-ML: Machine Learning Support for [Ariel OS](https://github.com/ariel-os/ariel-os/)
===

**!! Caution: Under construction, do not use in production !!**


# Installing the prerequisites

0. It is recommanded to use [miniconda](https://www.anaconda.com/docs/getting-started/miniconda/main) to set up the basic environment for reproducibility. You can find the `environment.yaml` file under this repo. Run `conda env create --file environment.yml` to get the same env as I have.
1. Get Ariel OS ready. Please follow this [official instruction](https://ariel-os.github.io/ariel-os/dev/docs/book/getting-started.html#installing-the-build-prerequisites).
2. Get Eerie ready. Here we use a variant of it: https://github.com/zhaolanhuang/eerie. Fetch the repo, then switch to `wip/ariel_ml` branch. (Run `git switch wip/ariel_ml`).
3. Get IREE ready.  






References
---

- [Ariel OS](https://github.com/ariel-os/ariel-os/)
- [Eerie: Rustic bindings to the IREE Compiler/Runtime](https://github.com/gmmyung/eerie) and the [variant](https://github.com/zhaolanhuang/eerie) adapted for Ariel OS.