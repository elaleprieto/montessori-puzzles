# Montessori Puzzles

Herramienta que toma la ilustración de una figura (un animalito, por ejemplo) y genera un rompecabezas Montessori con su forma, listo para cortar y grabar con láser.

## Language

### Entrada

**Ilustración**:
La imagen que carga el usuario (PNG, JPG o SVG) con una figura sobre fondo transparente o liso.
_Avoid_: foto, dibujo, input

**Silueta**:
El contorno exterior de la figura de la Ilustración, separado del fondo (o, con Silueta rectangular, el de la Ilustración entera). Es el borde exterior del Rompecabezas.
_Avoid_: outline, borde, máscara

**Silueta rectangular**:
La Silueta que sigue el borde de la Ilustración entera en lugar del de la figura, para escenas que llenan la imagen. No usa Umbral ni Margen; sus esquinas pueden ir redondeadas con un mismo radio en mm.
_Avoid_: marco, recuadro, modo rectángulo

**Umbral**:
El valor que decide qué parte de la Ilustración es figura y qué parte es fondo al detectar la Silueta.
_Avoid_: threshold, tolerancia

**Margen**:
La distancia en milímetros entre el borde de la figura dibujada y la Silueta que corta el láser.
_Avoid_: offset, padding, sangrado

### Rompecabezas

**Rompecabezas**:
El conjunto de Piezas que, encastradas, reconstruyen la Silueta.
_Avoid_: puzzle, juego

**Pieza**:
Cada una de las partes separables del Rompecabezas.
_Avoid_: región, celda, tile

**Unión**:
El borde que comparten dos Piezas vecinas.
_Avoid_: costura, frontera, junta

**Pestaña**:
La saliente redondeada de una Unión que encastra una Pieza en su vecina.
_Avoid_: knob, tab, encastre, oreja

**Cuadrícula**:
La forma de partir una Silueta rectangular en Filas × Columnas Piezas iguales, en lugar de Piezas irregulares.
_Avoid_: grilla, grid, modo regular

**Variante**:
Una de las particiones posibles de la misma Silueta en la misma cantidad de Piezas. Se identifica por su semilla. En Cuadrícula la partición es una sola: la semilla solo cambia hacia dónde salen las Pestañas y su irregularidad.
_Avoid_: versión, intento

### Salida

**Capa de corte**:
Las líneas que el láser atraviesa: la Silueta y todas las Uniones.
_Avoid_: cut layer, trazado

**Capa de grabado**:
La Ilustración, que el láser marca en la superficie sin atravesarla.
_Avoid_: engrave layer, dibujo, raster

**Ancho**:
El ancho físico de la Ilustración en el Rompecabezas, en milímetros. El alto sale de su proporción; el Rompecabezas lo excede solo por el Margen.
_Avoid_: tamaño, escala
