# Partición en Piezas sobre la grilla de píxeles

La Silueta se divide en Piezas trabajando sobre la máscara de píxeles y no con geometría de polígonos. Cada píxel de la figura se asigna a la semilla más cercana por distancia geodésica (caminando solo por dentro de la figura) y unas iteraciones de Lloyd emparejan las áreas. Después, los bordes entre etiquetas (incluido el fondo) se extraen como un único grafo de polilíneas, del que salen tanto la Silueta como las Uniones.

## Considered Options

- **Voronoi geométrico recortado contra la Silueta**: con siluetas cóncavas (trompa, cola, patas) una celda puede quedar partida en dos pedazos separados, y empatar los bordes recortados con la Silueta suavizada es frágil.
- **Grilla de píxeles (elegida)**: cada Pieza queda conexa por construcción, las áreas se emparejan con facilidad y como Silueta y Uniones salen del mismo grafo, sus extremos coinciden exactamente.

## Consequences

La precisión depende de la resolución de la grilla de trabajo; los bordes se suavizan y se convierten en curvas antes de exportar. El Margen se aplica dilatando la máscara, no desplazando polígonos.
