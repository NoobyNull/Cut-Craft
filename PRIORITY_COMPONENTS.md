# Priority Components - Largest Wind First

## 1. Core Architecture & Graphics Engine Integration (wgpu/WebGPU)
**Impact:** Highest
**Why:** This is the foundation that will enable all other functionality. Without proper graphics rendering, we can't visualize anything.
**Key Tasks:**
- Set up wgpu context and device
- Create basic 3D window with winit
- Implement simple scene rendering (cube or basic geometry)
- Basic camera controls (pan/zoom/orbit)

## 2. Basic UI Framework
**Impact:** High
**Why:** Users need to interact with the application. A minimal working interface is essential for any progress.
**Key Tasks:**
- Create main window structure
- Implement basic menu system
- Add viewport area for 3D rendering
- Basic status bar

## 3. File I/O and Project Management
**Impact:** High
**Why:** Users need to load/save models and work with projects. This is core functionality.
**Key Tasks:**
- Implement file loading (SVG, DXF)
- Create project structure management
- Add basic save/load functionality
- Support for common CAD file formats

## 4. Basic 3D Modeling Tools
**Impact:** Medium-High
**Why:** Core functionality of a CAD application - creating and manipulating geometry.
**Key Tasks:**
- Implement basic shape creation (boxes, cylinders)
- Simple transformation tools (translate, rotate, scale)
- Basic selection mechanisms
- Object hierarchy management

## 5. Directory Structure Management
**Impact:** Medium
**Why:** Essential for the application to function properly and store data.
**Key Tasks:**
- Create required directories (assets, config, output)
- Handle path resolution
- Implement proper error handling for file operations

## 6. Input Handling System
**Impact:** Medium
**Why:** Users need to interact with the interface through mouse/keyboard.
**Key Tasks:**
- Mouse event processing
- Keyboard shortcuts
- Basic interaction modes (select, move, etc.)