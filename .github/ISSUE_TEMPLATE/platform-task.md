---
name: 🏗️ Platform Engineering Task
about: Partition layout, memory allocation, flash/RAM constraints, hardware config
title: '[Platform] '
labels: ['persona:platform', 'hardware', 'memory']
assignees: ''

---

## Task Description
<!-- Brief description of the platform / memory / partition work -->

## Success Criteria
<!-- What needs to be completed for this task to be done? -->
- [ ] 
- [ ] 
- [ ] 

## Technical Requirements
<!-- Technical specifications, constraints, or dependencies -->

### Platform Scope
- Hardware target: 
- Flash size: 
- RAM constraints: 
- Partition changes needed:

### Memory Requirements
- Heap size: 
- Stack requirements: 
- Static allocation needs:
- DMA buffers:

### Cross-Reference Requirements
<!-- Does this require coordination with Build Systems Engineer? -->
- [ ] Coordinate with Build Systems Engineer on binary size impact
- [ ] Coordinate with Backend Developer on memory allocation in code
- [ ] Other cross-domain coordination:

## Definition of Done
- [ ] Partition layout validated and documented
- [ ] Memory allocation meets requirements
- [ ] Flash/RAM constraints verified
- [ ] Hardware configuration tested on target device
- [ ] Documentation updated (partition tables, memory maps)
- [ ] OTA compatibility verified (if partition changes affect OTA)
- [ ] No regression in boot stability

## Partition/Memory Details
<!-- For partition or memory allocation tasks -->

### Current State
- Current partition layout: 
- Current memory usage: 
- Identified constraints:

### Target State
- Proposed partition changes: 
- Expected memory savings/allocation:
- Trade-offs and risks:

## Hardware Context
<!-- ESP32C6 specific constraints, bootloader requirements, etc. -->

## Notes
<!-- Additional context, dependencies, or coordination notes -->

