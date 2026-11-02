# Cut Craft - Component Outline

**Status 2026-11-01 (version 0.4.2):** everything below is built except the items marked
*(not built)* or *(out of scope)*. The current list of work is `docs/ROADMAP.md`; the
tool-by-tool state is `docs/tool-inventory.md`.

## Core Architecture
- Application framework and lifecycle management
- Configuration system
- File I/O operations
- Directory structure management
- Logging and error handling

## Graphics & Rendering
- 3D graphics engine integration (wgpu/WebGPU)
- Scene rendering pipeline
- Model visualization
- Camera controls
- Lighting and materials
- Viewport management

## CAD Functionality
- 3D modeling tools
- Geometry creation and manipulation
- Toolpath generation
- CNC machining simulation
- Mesh operations
- Boolean operations

## UI Components
- Main window interface
- Toolbar and menus
- Property panels
- Status bar
- Viewport controls
- Dialog windows
- File dialogs

## Data Management
- File format support (SVG, DXF, STL, etc.)
- Project management
- Undo/redo functionality
- Preferences system
- Data serialization

## Performance & Optimization
- Memory management
- Rendering optimization
- Multi-threading support
- Resource loading
- Caching mechanisms

## Input Handling
- Mouse and keyboard input
- Touch gestures (for tablets) *(not built)*
- Hotkeys and shortcuts
- Event processing

## Export Functionality
- G-code generation
- File export capabilities
- Post-processing
- Machine-specific formats

## Advanced Features
- Plugin architecture *(out of scope unless decided: see ROADMAP, Decisions)*
- Scripting support: Lua 5.4 gadgets in `crates/scripting` (sandboxed, one undo step per run; gadgets will install as add-ons)
- Advanced modeling tools
- Simulation and verification