// Add PDF download button to mdBook toolbar
(function() {
    var rightButtons = document.querySelector('.right-buttons');
    if (!rightButtons) return;
    var pdfLink = document.createElement('a');
    pdfLink.href = 'practical-biostatistics-in-30-days.pdf';
    pdfLink.title = 'Download PDF';
    pdfLink.innerHTML = '<i class="fa fa-file-pdf-o" style="font-size:1.2em;"></i>';
    pdfLink.style.cssText = 'text-decoration:none;font-size:1.2em;padding:0 4px;';
    rightButtons.insertBefore(pdfLink, rightButtons.firstChild);
})();
