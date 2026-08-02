# Turnstiles

<a href="https://github.com/Zylatis/turnstiles/actions/workflows/rust.yml"><img src="https://github.com/Zylatis/turnstiles/actions/workflows/rust.yml/badge.svg" /></a>
<a href="https://crates.io/crates/turnstiles"><img src="https://raster.shields.io/crates/v/turnstiles.png" /></a>

Una biblioteca en desarrollo (WIP) que envuelve el trait `io::Write` para permitir la rotación de archivos, es decir, para registros (logs). El objetivo es permitir la rotación de archivos a nivel de manejador de archivos y hacerlo con la menor cantidad de dependencias posible.

Condiciones de rotación implementadas/planificadas:
- [x] Ninguna (nunca rotar)
- [x] SizeMB (tamaño del archivo)
- [x] Duration (tiempo desde la última modificación)
- [ ] SizeLines (número de líneas en el archivo)

También hay tres opciones para podar los registros antiguos:
- [x] Ninguna
- [x] MaxFiles
- [x] MaxAge

## Advertencia:
Esto se encuentra actualmente en desarrollo activo y puede cambiar o romperse con frecuencia. Se hará todo lo posible para garantizar que los cambios que rompan la compatibilidad se reflejen en un cambio de al menos la versión menor del paquete, tanto en términos de la API como de la generación de archivos de registro. Las versiones anteriores a la 0.2.0 estaban tan llenas de errores que me asombra que haya logrado ponerme los pantalones en los días que lo estaba escribiendo.

# Documentación
Consulta la documentación [aquí](https://docs.rs/turnstiles/latest/turnstiles/) para notas sobre cómo funciona, ejemplos de uso y manejo de errores.

## Trabajo futuro
- Refrescar el índice interno cuando se solicite rotación, no solo al crear el registrador
- Tener más cuidado con los casos límite, por ejemplo rotar archivos de 1 MB y escribir exactamente 1 MB en disco
- Integración más directa con bibliotecas de registro dedicadas, es decir, `impl log::Log`.
- Investigar la integración con cosas como [`atomicwrites`](https://crates.io/crates/atomicwrites)
- Opciones de rotación más flexibles
## ¿Por qué `turnstiles`?
Es una cosa de metal que rota, y también es el nombre del álbum de Billy Joel que tiene [`Summer, Highland Falls`](https://youtu.be/WsNhuJypNjM), una de mis canciones favoritas.
