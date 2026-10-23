# AMD Graphics Support in Cut Craft

## Overview

Cut Craft is designed to support AMD graphics hardware through the wgpu graphics library, which provides cross-platform rendering capabilities including Vulkan support that AMD GPUs excel at.

## Implementation Details

### Backend Selection
The project uses `wgpu::Backends::PRIMARY` which includes:
- Vulkan (Primary choice for AMD GPUs)
- DirectX 12 (for Windows)
- Metal (for macOS)
- OpenGL (fallback)

### Device Selection Process
1. The application requests an adapter from wgpu
2. wgpu automatically selects the best available backend
3. For AMD systems, this typically resolves to Vulkan
4. Proper error handling ensures compatibility across different hardware

## Testing AMD Support

To verify AMD graphics support:

1. Run the application on an AMD system
2. Check that no graphics errors occur during initialization
3. Confirm that the application can create a rendering context

## Benefits for AMD Users

- **Vulkan Support**: AMD GPUs have excellent Vulkan driver support
- **Performance**: Vulkan provides low-level access for optimal performance
- **Compatibility**: Cross-platform compatibility with various AMD GPU generations
- **Future-Proof**: Modern graphics API support

## Troubleshooting

If you encounter issues on AMD hardware:
1. Ensure you have up-to-date graphics drivers
2. Verify Vulkan runtime is installed (`vulkan-tools`, `vulkan-validation-layers`)
3. Check that your GPU is supported by the wgpu library

## Future Enhancements

- Add explicit Vulkan backend selection for better AMD optimization
- Implement advanced AMD-specific rendering features
- Performance profiling on different AMD GPU models