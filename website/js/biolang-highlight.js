// Custom highlight.js language definition for BioLang (.bl)
window.registerBioLang = function (hljs) {
  hljs.registerLanguage('biolang', function () {
    return {
      name: 'BioLang',
      aliases: ['bl', 'biolang'],
      keywords: {
        keyword: 'let fn return if else match for in while break continue import from as pub struct enum trait impl use module where type true false nil',
        built_in: 'print println len type range abs min max int float str bool map filter reduce sort push pop zip enumerate flatten reverse unique first last sum mean median stdev count keys values contains split join trim upper lower replace starts_with ends_with regex_match regex_find regex_replace read_text write_text read_csv write_csv read_json write_json read_fasta read_fastq read_bed read_gff read_vcf read_sam read_bam read_maf write_fasta write_fastq write_bed write_vcf select arrange group_by summarize mutate left_join inner_join pivot_wider pivot_longer window transpose dot pca cor_matrix matrix zeros eye sparse_matrix gc_content reverse_complement transcribe translate kmer_count align motif_find consensus edit_distance hamming_distance interval intersect merge subtract http_get http_post download upload sparkline bar_chart boxplot plot heatmap histogram volcano manhattan qq_plot ideogram circos oncoprint chat chat_code par_map par_filter await_all',
        type: 'Int Float String Bool List Map Table Matrix Seq Interval DNA RNA Protein Bed Vcf Gff Sam Bam Option Result',
        literal: 'true false nil'
      },
      contains: [
        hljs.HASH_COMMENT_MODE,
        hljs.QUOTE_STRING_MODE,
        {
          className: 'string',
          begin: 'dna"', end: '"',
          contains: [
            { className: 'template-variable', begin: '[ATGCatgc]+' }
          ]
        },
        {
          className: 'string',
          begin: 'rna"', end: '"'
        },
        {
          className: 'string',
          begin: 'protein"', end: '"'
        },
        hljs.C_NUMBER_MODE,
        {
          className: 'operator',
          begin: /\|>/
        },
        {
          className: 'operator',
          begin: /=>|->|\.\./
        },
        {
          className: 'function',
          beginKeywords: 'fn',
          end: /\{/,
          excludeEnd: true,
          contains: [
            hljs.UNDERSCORE_TITLE_MODE,
            {
              className: 'params',
              begin: /\(/, end: /\)/,
              contains: [hljs.C_NUMBER_MODE, hljs.QUOTE_STRING_MODE]
            }
          ]
        }
      ]
    };
  });
};
