import pandas as pd, scanpy as sc, anndata, numpy as np
sc.settings.verbosity = 0
df = pd.read_csv('/data/counts.csv.gz', index_col=0)      # genes x cells
adata = anndata.AnnData(df.T.astype('float32'))            # cells x genes
sc.pp.filter_cells(adata, min_genes=200)
sc.pp.filter_genes(adata, min_cells=3)
sc.pp.subsample(adata, n_obs=500, random_state=0)          # tractable subset
adata.layers['counts'] = adata.X.copy()
sc.pp.normalize_total(adata, target_sum=1e4)
sc.pp.log1p(adata)
sc.pp.highly_variable_genes(adata, n_top_genes=1200)
adata = adata[:, adata.var.highly_variable].copy()
# reference pipeline (seeded → reproducible)
sc.pp.pca(adata, n_comps=30, random_state=0)
sc.pp.neighbors(adata, n_neighbors=15, random_state=0)
sc.tl.leiden(adata, resolution=1.0, random_state=0)
sc.tl.rank_genes_groups(adata, 'leiden', method='wilcoxon')
# export RAW subset (same cells x HVG) for BioLang
raw = adata.layers['counts']
raw = np.asarray(raw.todense()) if hasattr(raw, 'todense') else np.asarray(raw)
np.savetxt('/data/_real_matrix.tsv', raw, delimiter='\t', fmt='%g')
open('/data/_real_genes.txt','w').write('\n'.join(map(str, adata.var_names)))
open('/data/_real_barcodes.txt','w').write('\n'.join(map(str, adata.obs_names)))
adata.obs['leiden'].to_csv('/data/_ref_labels.csv', index=False, header=False)
nm = adata.uns['rank_genes_groups']['names']
with open('/data/_ref_markers.csv','w') as f:
    for g in nm.dtype.names:
        f.write(g + ',' + ','.join(str(nm[g][i]) for i in range(5)) + '\n')
print('PREP_OK', adata.n_obs, 'cells x', adata.n_vars, 'HVG', adata.obs['leiden'].nunique(), 'clusters')
