#version 460
#extension GL_ARB_gpu_shader_int64 : enable

#define u32 uint
#define u64 uint64_t

layout(local_size_x = 16, local_size_y = 8, local_size_z = 1) in;

layout(push_constant) uniform Params {
   u32 offset;
} params;

// input[] on the CPU side, fixe
layout(std430, binding = 0) buffer layout1 { u32 inputReadOnly[16]; };
// final checksums & result index / data1007 value
layout(std430, binding = 1) buffer layout0 { u32 checksumsRes[4]; };

#define MAGIC_NUMBER  0x6c078965

// Assumes that the difference check will never be zero.
// This may give wrong results (false positives & negatives).
#define FAST_HASH_MUL 1
// Assumes that finalizeHigh's last hash buff.y value will never be zero.
#define FAST_FINALIZE_HIGH 1

/*
u32 rotr(u32 n, u32 d) {
    return (n >> d)|(n << (32 - d));
}
*/
u32 rotr(u32 x, u32 n) {
  return u32(packUint2x32(uvec2(x)) >> n);
}

u32 mul64bit_diff(u32 x, u32 y) {
    // using 64-bit integer is way faster then imulExtended
    uvec2 parts = unpackUint2x32(u64(x) * u64(y));
    return parts.y - parts.x;
}

u32 hashMulDiff(u32 factorBase, u32 factorA, u32 factorB) {
    if (factorA == 0)factorA = factorB;
    u32 diff = mul64bit_diff(factorBase, factorA);

    #ifdef FAST_HASH_MUL
      return diff;
    #else
      return (diff == 0) ? factorBase : diff;
    #endif
}

u32 hashMulDiff_ANonZero(u32 factorBase, u32 factorA) {
    u32 diff = mul64bit_diff(factorBase, factorA);

    #ifdef FAST_HASH_MUL
      return diff;
    #else
      return (diff == 0) ? factorBase : diff;
    #endif
}

void finalizeLow_Step(inout uvec2 buf, u32 data, u32 i)
{
  u32 tmp = (data & 0x02) >> 1;
  u32 tmp2 = data & 0x01;

  if (tmp == tmp2) {
    buf[0] += data;
  }  else {
    buf[0] = hashMulDiff(buf[0], data, i);
  }

  if(tmp2 == 1) {
    buf[1] ^= data;
  } else {
    buf[1] = hashMulDiff(buf[1], data, i);
  }
}

u32 finalizeLow(in u32[16] state)
{
    uvec2 buf = uvec2(state[0]);

    finalizeLow_Step(buf, state[0], 0);
    buf[1] = hashMulDiff_ANonZero(buf[1], 1);
    finalizeLow_Step(buf, state[2], 2);
    finalizeLow_Step(buf, state[3], 3);
    finalizeLow_Step(buf, state[4], 4);
    finalizeLow_Step(buf, state[5], 5);
    finalizeLow_Step(buf, state[6], 6);
    buf[1] = hashMulDiff_ANonZero(buf[1], 7);
    buf[1] = hashMulDiff_ANonZero(buf[1], 8);
    finalizeLow_Step(buf, state[9], 9);
    finalizeLow_Step(buf, state[10], 10);
    finalizeLow_Step(buf, state[11], 11);
    buf[1] = hashMulDiff_ANonZero(buf[1], 12);
    finalizeLow_Step(buf, state[13], 13);
    buf[1] = hashMulDiff_ANonZero(buf[1], 14);
    buf[1] = hashMulDiff_ANonZero(buf[1], 15);

    return buf[0] ^ buf[1];
}

void finalizeHigh_Step(inout uvec2 buf, u32 data, u32 i)
{
    buf.x += rotr(data, data & 0x1F);

    // branchless version is slightly faster
    u32 branchA = data < buf.x ? 1 : 0;
    u32 branchB = 1 - branchA;

    #ifdef FAST_FINALIZE_HIGH
      buf.y = (u32(branchA) * (buf.y + data))
            + (branchB * hashMulDiff_ANonZero(buf.y, data));
    #else
      buf.y = (branchA * (buf.y + data))
            + (branchB * hashMulDiff(buf.y, data, i));
    #endif
}

u32 finalizeHigh(in u32[16] state)
{
    uvec2 buf = uvec2(state[0]);

    finalizeHigh_Step(buf, state[0], 0);
    //if (buf[0] == 0)buf[1] = -buf[1]; // UNLIKELY, may cause false positives
    finalizeHigh_Step(buf, state[2], 2);
    finalizeHigh_Step(buf, state[3], 3);
    finalizeHigh_Step(buf, state[4], 4);
    finalizeHigh_Step(buf, state[5], 5);
    finalizeHigh_Step(buf, state[6], 6);
     /*if (buf[0] == 0) { // UNLIKELY, may cause false positives
      buf[1] = hashMulDiff_ANonZero(buf[1], 7);
      buf[1] = hashMulDiff_ANonZero(buf[1], 8);
    }*/
    finalizeHigh_Step(buf, state[9], 9);
    finalizeHigh_Step(buf, state[10], 10);
    finalizeHigh_Step(buf, state[11], 11);

    // state[12] is forced to be zero:
    // finalizeHigh_Step(buf, state[12], 12);
    // here would be the correct way to handle it with zero:
    /*if (buf[0] == 0) { // UNLIKELY, may cause false positives
      buf[1] = hashMulDiff_ANonZero(buf[1], 12);
    }*/

    finalizeHigh_Step(buf, state[13], 13);

    // skip last two steps, since they will reuslt in either zero
    // or create a situation where buf[1] wouldn't change
    /*if (buf[0] == 0) {
      buf[1] = hashMulDiff_ANonZero(buf[1], 14);
      buf[1] = hashMulDiff_ANonZero(buf[1], 15);
    }*/

    #ifdef FAST_FINALIZE_HIGH
      return hashMulDiff_ANonZero(buf.x, buf.y) & 0xFFFF;
    #else
      return hashMulDiff(buf.x, buf.y, 16) & 0xFFFF;
    #endif
}

void checksumStep_1007_1008(inout u32[16] state, u32 data1007)
{
  // Step 1008: dataLast is always zero, data1007 is never zero
  //state[0] += hashMulDiff_ANonZero(-1, data1007);
  state[0] += data1007 + data1007 - 1; // same as above

  state[2] ^= data1007;
  state[3] += hashMulDiff_ANonZero(data1007 + 5, MAGIC_NUMBER);

  state[4] += data1007;
  state[5] += data1007;

  if (data1007 < state[6]) {
    state[6] = (state[3] + state[6]) ^ (data1007 + 1008);
  } else {
    state[6] ^= (state[4] + data1007);
  }

  state[9] = hashMulDiff_ANonZero(state[9], data1007);

  // Step 1007: data & dataLast is always zero, data1007 is never zero
  state[10] = hashMulDiff_ANonZero(state[10], data1007);
  state[11] = hashMulDiff_ANonZero(state[11], data1007);

  state[13] += rotr(data1007, data1007 & 0x1F);
}

void main()
{
  const u32 id = gl_GlobalInvocationID.x
    + gl_GlobalInvocationID.y * gl_NumWorkGroups.x * gl_WorkGroupSize.x;

  // state is shared acrosss workers and even acrosss invocations, make copy
  u32 state[16];
  for(int i=0; i<16; ++i)state[i] = inputReadOnly[i];

  // last 2 steps in the checksum
  u32 data1007 = id + params.offset; // would be input[1007] on the CPU
  checksumStep_1007_1008(state, data1007);

  // finalize and write out checksums if it matches
  u32 high = finalizeHigh(state);
  //if(high == (id&0xFFFF)) // (DEBUG)
  if(high == 0x00008618)
  {
    u32 low = finalizeLow(state);
    //return true;
    //if((low & 0xFFFF) == 0xC2D3) // 16 bits (DEBUG)
    //if((low & 0xFFFFF) == 0xBC2D3) // 20 bits (DEBUG)
    //if((low & 0xFFFFFF) == 0x5BC2D3) // 24 bits (DEBUG)
    //if((low & 0xFFFFFFF) == 0x45BC2D3) // 28 bits (DEBUG)
    if(low == 0xA45BC2D3) // 32 bits (full)
    {
      checksumsRes[0] = finalizeLow(state);
      checksumsRes[1] = finalizeHigh(state);
      checksumsRes[2] = data1007;
      checksumsRes[3] = id;
    }
  }
}