# Li-Fi Protocol Specification for Housaky

## Overview
Light Fidelity (Li-Fi) communication protocol for Housaky AGI network. Uses LED transmission and camera reception to create optical mesh network.

## Design Goals
- **Throughput**: 1-100 kbps (initial-optimized)
- **Latency**: <50ms for local communication
- **Range**: 1-10 meters line-of-sight
- **Reliability**: 99%+ packet delivery with error correction
- **Coexistence**: Work alongside RF networks
- **Security**: Physically isolated (light doesn't penetrate walls)

## Physical Layer

### Transmitter (LED)
- **Wavelength**: IR 850nm (primary), visible 450-650nm (debug)
- **Modulation**: On-Off Keying (OOK)
- **Symbol Rate**: 1 kHz (initial), 10 kHz (optimized)
- **Power**: 100-500 mW per LED
- **Array**: 4 LEDs for spatial multiplexing

**Circuit Design**:
```
GPIO Pin → MOSFET (IRLZ44N) → LED (850nm, 1W) → Current Limiting Resistor → GND
                                    ↓
                              Heatsink (required)
```

**Modulation Scheme**:
- `1` = LED ON (100% duty cycle)
- `0` = LED OFF (0% duty cycle)
- Manchester encoding for clock recovery

### Receiver (Camera)
- **Sensor**: CMOS rolling shutter (exploited for high-speed capture)
- **Frame Rate**: 60 fps (minimum), 120 fps (recommended)
- **Resolution**: 720p (sufficient for LED detection)
- **Exposure**: Low (1-5ms) to avoid motion blur
- **ROI**: Track LED positions, process only relevant pixels

**Detection Algorithm**:
1. Background subtraction (remove ambient light)
2. Blob detection (find LED sources)
3. Intensity thresholding (binary decision)
4. Temporal sampling (extract bit stream)
5. Manchester decoding (recover clock + data)

## Data Link Layer

### Frame Format
```
┌──────────┬──────────┬──────────┬──────────┬──────────┬──────────┐
│ Preamble │  Header  │  Payload │   CRC    │   FEC    │   EOF    │
│  8 bytes │  8 bytes │ 0-256 B  │  4 bytes │ 32 bytes │  4 bytes │
└──────────┴──────────┴──────────┴──────────┴──────────┴──────────┘
```

**Preamble** (8 bytes):
- Sync pattern: `0xAA 0xAA 0xAA 0xAA 0x55 0x55 0x55 0x55`
- Purpose: Clock synchronization, frame detection

**Header** (8 bytes):
```
┌──────────┬──────────┬──────────┬──────────┬──────────┬──────────┐
│  Version │   Type   │  Length  │ Sequence │  LED ID  │ Reserved │
│  1 byte  │  1 byte  │  2 bytes │  2 bytes │  1 byte  │  1 byte  │
└──────────┴──────────┴──────────┴──────────┴──────────┴──────────┘
```
- Version: Protocol version (0x01)
- Type: Frame type (DATA, ACK, BEACON, etc.)
- Length: Payload length (0-256)
- Sequence: Frame sequence number (for ordering)
- LED ID: Which LED transmitted (0-3 for spatial multiplexing)
- Reserved: Future use

**Payload** (0-256 bytes):
- Application data

**CRC** (4 bytes):
- CRC-32 checksum for error detection

**FEC** (32 bytes):
- Reed-Solomon (255, 223) error correction
- Can recover up to 16 byte errors

**EOF** (4 bytes):
- End-of-frame marker: `0xFF 0xFF 0xFF 0xFF`

### Frame Types
- `0x01` DATA: User data
- `0x02` ACK: Acknowledgment
- `0x03` NACK: Negative acknowledgment
- `0x04` BEACON: Presence announcement
- `0x05` ROUTE_REQ: Route discovery
- `0x06` ROUTE_REP: Route reply

### Error Control

**Error Detection**:
- CRC-32 for all frames
- Discard frames with CRC mismatch

**Error Correction**:
- Reed-Solomon (255, 223) forward error correction
- Corrects up to 16 byte errors per frame
- Reduces retransmission overhead

**ARQ (Automatic Repeat Request)**:
- Stop-and-wait for simplicity
- Sender waits for ACK before sending next frame
- Timeout: 100ms (adjustable based on distance)
- Max retries: 3

### Flow Control
- Stop-and-wait (simple, low throughput)
- Future: Sliding window (higher throughput)

## Network Layer

### Addressing
- **Node ID**: 128-bit (derived from ed25519 public key)
- **Short Address**: 16-bit (for efficiency, mapped from Node ID)
- **Broadcast**: `0xFFFF`

### Routing Protocol
**Distance Vector with Light-Path Optimization**:

1. **Neighbor Discovery**:
   - Periodic BEACON frames (every 5 seconds)
   - Contains: Node ID, battery level, capabilities
   - Neighbors respond with BEACON

2. **Route Discovery**:
   - Source sends ROUTE_REQ (broadcast)
   - Intermediate nodes forward + record reverse path
   - Destination sends ROUTE_REP (unicast back)
   - Source caches route

3. **Route Maintenance**:
   - Periodic route refresh (every 60 seconds)
   - Detect broken links (missed BEACONs)
   - Trigger route rediscovery

4. **Light-Path Optimization**:
   - Prefer routes with:
     - Fewer hops (lower latency)
     - Higher light intensity (better SNR)
     - Lower ambient noise (fewer errors)
     - Higher battery levels (reliability)

### Packet Format
```
┌──────────┬──────────┬──────────┬──────────┬──────────┐
│  Src ID  │  Dst ID  │   TTL    │   Hops   │  Payload │
│ 16 bits  │ 16 bits  │  8 bits  │  8 bits  │ Variable │
└──────────┴──────────┴──────────┴──────────┴──────────┘
```

### Fragmentation
- MTU: 256 bytes (data link layer limit)
- Larger packets fragmented at network layer
- Reassembly at destination

## Transport Layer

### Reliable Stream (TCP-like)
- Connection-oriented
- Three-way handshake (SYN, SYN-ACK, ACK)
- Sliding window flow control
- Congestion control (AIMD)
- In-order delivery

### Unreliable Datagram (UDP-like)
- Connectionless
- No guarantees
- Lower overhead
- Use for: Beacons, telemetry, non-critical data

## Application Layer

### Photon State Protocol
Custom protocol for transmitting quantum-inspired photon states:

```
┌──────────┬──────────┬──────────┬──────────┬──────────┐
│   Type   │Timestamp │ Location │  State   │ Metadata │
│  1 byte  │  8 bytes │  8 bytes │ 32 bytes │ Variable │
└──────────┴──────────┴──────────┴──────────┴──────────┘
```

**State** (32 bytes):
- Polarization: 3x float32 (Stokes parameters S0, S1, S2)
- Intensity: 1x float32
- Phase: 1x float32
- Wavelength: 1x float32
- Coherence: 1x float32
- Entanglement: 1x float32 (correlation with other photons)

### Code Improvement Protocol
For broadcasting self-improvement updates:

```
┌──────────┬──────────┬──────────┬──────────┬──────────┐
│   Type   │Generation│Performance│  Patch   │Signature │
│  1 byte  │  8 bytes │  8 bytes │ Variable │ 64 bytes │
└──────────┴──────────┴──────────┴──────────┴──────────┘
```

## Performance Optimization

### Adaptive Modulation
- Monitor bit error rate (BER)
- If BER > 10^-3: Reduce symbol rate (more robust)
- If BER < 10^-5: Increase symbol rate (higher throughput)

### Spatial Multiplexing
- Use 4 LEDs simultaneously
- Each LED transmits different data stream
- Camera tracks each LED separately
- 4x throughput increase

### Temporal Multiplexing
- Exploit camera rolling shutter
- Each row captures different time slice
- Effective sampling rate = frame_rate × num_rows
- Example: 60 fps × 720 rows = 43.2 kHz sampling

### Ambient Light Cancellation
- Capture background frame (LEDs off)
- Subtract from data frames (LEDs on)
- Removes sunlight, room lights, etc.

## Security

### Physical Security
- Light doesn't penetrate walls (inherent isolation)
- Eavesdropping requires line-of-sight
- Jamming requires bright light source (detectable)

### Cryptographic Security
- Noise protocol (XX handshake) for key exchange
- ChaCha20-Poly1305 for encryption + authentication
- Ed25519 for signatures
- BLAKE3 for hashing

### Anti-Jamming
- Frequency hopping (switch LED wavelengths)
- Spread spectrum (encode data across multiple LEDs)
- Directional transmission (focus light beam)

## Implementation Roadmap

### Phase 1: Basic Transmission (Week 1-2)
- [ ] LED driver circuit
- [ ] GPIO control (Raspberry Pi)
- [ ] OOK modulation (1 kHz)
- [ ] Camera capture (60 fps)
- [ ] Blob detection
- [ ] Bit stream extraction

### Phase 2: Reliable Link (Week 3-4)
- [ ] Frame formatting
- [ ] CRC-32 error detection
- [ ] Reed-Solomon FEC
- [ ] Stop-and-wait ARQ
- [ ] Throughput: 1 kbps

### Phase 3: Networking (Week 5-6)
- [ ] Neighbor discovery (BEACON)
- [ ] Route discovery (ROUTE_REQ/REP)
- [ ] Packet forwarding
- [ ] Multi-hop routing

### Phase 4: Optimization (Week 7-8)
- [ ] Adaptive modulation
- [ ] Spatial multiplexing (4 LEDs)
- [ ] Temporal multiplexing (rolling shutter)
- [ ] Ambient light cancellation
- [ ] Throughput: 10+ kbps

### Phase 5: Integration (Week 9-10)
- [ ] libp2p custom transport
- [ ] Photon state protocol
- [ ] Code improvement protocol
- [ ] Full system testing

## Testing & Validation

### Unit Tests
- Frame encoding/decoding
- CRC calculation
- Reed-Solomon FEC
- Manchester encoding

### Integration Tests
- LED → Camera loopback
- Multi-hop routing (3+ nodes)
- Packet loss scenarios
- Ambient light interference

### Performance Tests
- Throughput (kbps)
- Latency (ms)
- Bit error rate (BER)
- Packet loss rate (PLR)
- Range (meters)

### Real-World Tests
- Indoor (office, home)
- Outdoor (sunlight)
- Mobile (moving nodes)
- Interference (other light sources)

## Hardware Bill of Materials

### Per Node
- 4x IR LED (850nm, 1W, 100° beam angle) - $2 each
- 4x MOSFET (IRLZ44N or similar) - $1 each
- 4x Current limiting resistor (calculated based on LED) - $0.10 each
- 1x Heatsink for LEDs - $5
- 1x USB camera (720p @ 60fps minimum) - $20
- 3x Linear polarizing filter (0°, 45°, 90°) - $5 each
- 1x Raspberry Pi 5 (8GB) or equivalent - $80
- 1x Breadboard + jumper wires - $10
- 1x Power supply (5V 3A for RPi + LEDs) - $10

**Total per node**: ~$150

### Optional Upgrades
- High-speed camera (120+ fps): +$50
- Collimating lens (focus LED beam): +$10 each
- Photodiode (faster than camera): +$5
- Solar panel (5W) + battery: +$30

## References

### Academic Papers
- "Visible Light Communication: Concepts, Applications and Challenges" (IEEE, 2015)
- "High-Speed Visible Light Communication Using RGB LED" (IEEE, 2018)
- "Camera-Based Visible Light Communication" (ACM, 2020)

### Standards
- IEEE 802.15.7: Short-Range Optical Wireless Communications
- ITU-T G.9991: High-speed indoor visible light communication

### Open Source Projects
- OpenVLC: Open-source VLC platform
- LiFi-LED: Arduino-based Li-Fi
- PyLiFi: Python Li-Fi library

## Future Enhancements

### Hardware
- Avalanche photodiodes (APD) for higher sensitivity
- Laser diodes for longer range (1+ km)
- Optical filters for wavelength division multiplexing
- Fresnel lenses for beam steering

### Protocol
- OFDM modulation (higher spectral efficiency)
- MIMO (multiple input multiple output)
- Cognitive radio (dynamic spectrum access)
- Quantum key distribution (QKD) over Li-Fi

### Applications
- Underwater communication (blue-green laser)
- Secure military communication
- Indoor positioning (VLP)
- Vehicle-to-vehicle (V2V) communication
