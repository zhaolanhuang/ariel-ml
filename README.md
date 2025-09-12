Ariel-ML: Machine Learning Support for [Ariel OS](https://github.com/ariel-os/ariel-os/)
===

**!! Caution: Under construction, do not use in production !!**


Installing the prerequisites
---

0. It is **recommended** to use [Miniconda](https://www.anaconda.com/docs/getting-started/miniconda/main) to set up the environment for reproducibility.
   All dependencies are listed in the `environment.yml` file included in this repository.
   Run the following command to create the same environment:

   ```bash
   conda env create --file environment.yml
   ```

1. **Set up Ariel OS**
   Follow the [official installation guide](https://ariel-os.github.io/ariel-os/dev/docs/book/getting-started.html#installing-the-build-prerequisites).

2. **Set up Eerie**
   We use a variant of Eerie: [zhaolanhuang/eerie](https://github.com/zhaolanhuang/eerie).
   Clone the repository and switch to the `wip/ariel_ml` branch:

   ```bash
   git switch wip/ariel_ml
   ```

   * **2.1 Install the IREE compiler**

     ```bash
     pip install iree-compiler
     ```

   * **2.2 Initialize submodules (IREE runtime)**
     The IREE runtime is embedded as a submodule under the Eerie repo.
     From the Eerie root, run:

     ```bash
     git submodule update --init --recursive
     ```

     ⚠️ This step may consume \~3 GiB of disk space.

3. **Configure package paths**

   * Update the `path` field in `laze-project.yml` to point to your cloned **Ariel OS** repository.
   * Update the Eerie package path in `Cargo.toml`.

Once these steps are complete, your playground environment is ready for experimenting with ML models.


Compile and Deploy Preset Models
---

Currently, we have tested two models under the **native** target:

* **Toy model:** `simple_mul`
* **Real-world model:** `resnet50`

You can select a model with the `--features` option:

```bash
laze build -b native run --features {resnet50,simple_mul}
```

The build system will automatically compile the chosen model.

## Notes on ResNet50

The model file `resnet50.mlir` is quite large (\~100 MB) and is not included in the repository.
You can either:

* Download it directly: [Google Drive link](https://drive.google.com/file/d/1xTnttlKH9YY6veXAF3NVZQlrFxMHuUCu/view?usp=sharing)
* Or generate your own version using the script:

  ```bash
  python load_mlir.py cat115.jpg
  ```

## Notes on simple_mul

This toy model performs an element-wise multiplication of two floating-point vectors (4 elements each).

⚠️ **Limitation:**
It cannot currently run with `no_std` under native.
The issue arises because the IREE runtime loads the model code into a memory section without execution permission (heap) and then attempts to execute it, causing a **SIGSEGV** on our testbed.

**Workaround:**
To run it successfully, change the Eerie dependency features in `Cargo.toml` to:

```toml
eerie = { path = "/path/to/eerie", features = ["runtime, std"], default-features = false}
```


References
---

- [Ariel OS](https://github.com/ariel-os/ariel-os/)
- [Eerie: Rustic bindings to the IREE Compiler/Runtime](https://github.com/gmmyung/eerie) and the [variant](https://github.com/zhaolanhuang/eerie) adapted for Ariel OS.