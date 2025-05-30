document.addEventListener('DOMContentLoaded', function() {
  // Apply zoom effect to all content images except those with the .no-zoom class
  const images = document.querySelectorAll('.book-page img:not(.no-zoom, .book-icon)');

  // Initialize Medium Zoom
  mediumZoom(images, {
    margin: 24,
    background: 'rgba(0, 0, 0, 0.9)',
    scrollOffset: 40
  });
});
