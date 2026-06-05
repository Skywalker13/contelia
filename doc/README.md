# Workflow Quarto : PDF LaTeX + Site statique

## Installation

```bash
# 1. Installer Quarto
#    → https://quarto.org/docs/get-started/

# 2. Installer une distribution LaTeX (pour le PDF)
#    Option légère recommandée :
quarto install tinytex

# 3. Vérifier
quarto check
```

## Commandes principales

```bash
# Prévisualiser le site en direct (rechargement automatique)
quarto preview

# Générer le site complet dans _site/
quarto render

# Générer seulement le PDF d'un article
quarto render articles/article-1.qmd --to pdf

# Générer seulement le HTML d'un article
quarto render articles/article-1.qmd --to html
```
