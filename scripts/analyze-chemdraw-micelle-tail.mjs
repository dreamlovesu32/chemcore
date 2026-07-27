import fs from "node:fs/promises";

const CIRCLE_CONTROL = 0.551_865_734_604_922_4;
const WAVE_CONTROL = 0.552_284_749_830_793_6;

function quarterPoint(t) {
  const inverse = 1 - t;
  return [
    inverse ** 3 + 3 * inverse ** 2 * t +
      3 * inverse * t ** 2 * CIRCLE_CONTROL,
    -(3 * inverse ** 2 * t * CIRCLE_CONTROL +
      3 * inverse * t ** 2 + t ** 3),
  ];
}

function quarterDerivative(t) {
  const inverse = 1 - t;
  return [
    3 * inverse ** 2 * (1 - 1) +
      6 * inverse * t * (CIRCLE_CONTROL - 1) +
      3 * t ** 2 * (0 - CIRCLE_CONTROL),
    3 * inverse ** 2 * (-CIRCLE_CONTROL) +
      6 * inverse * t * (-1 + CIRCLE_CONTROL) +
      3 * t ** 2 * (-1 + 1),
  ];
}

function integrateSpeed(end, slices = 256) {
  if (end <= 0) return 0;
  const count = slices + (slices % 2);
  const step = end / count;
  let sum = 0;
  for (let index = 0; index <= count; index++) {
    const [dx, dy] = quarterDerivative(index * step);
    const weight = index === 0 || index === count
      ? 1
      : index % 2 === 0
      ? 2
      : 4;
    sum += weight * Math.hypot(dx, dy);
  }
  return sum * step / 3;
}

const QUARTER_LENGTH = integrateSpeed(1);

function circleFrame(fraction) {
  const coordinate = fraction * 4;
  const quadrant = Math.floor(coordinate) % 4;
  const target = (coordinate - Math.floor(coordinate)) * QUARTER_LENGTH;
  let low = 0;
  let high = 1;
  for (let iteration = 0; iteration < 32; iteration++) {
    const middle = (low + high) / 2;
    if (integrateSpeed(middle, 64) < target) low = middle;
    else high = middle;
  }
  const t = (low + high) / 2;
  let point = quarterPoint(t);
  let tangent = quarterDerivative(t);
  for (let turn = 0; turn < quadrant; turn++) {
    point = [point[1], -point[0]];
    tangent = [tangent[1], -tangent[0]];
  }
  const tangentLength = Math.hypot(...tangent);
  tangent = tangent.map((value) => value / tangentLength);
  return {
    point,
    tangent,
    outward: [-tangent[1], tangent[0]],
  };
}

function circleFrameAtAngle(fraction) {
  const coordinate = fraction * 4;
  const quadrant = Math.floor(coordinate) % 4;
  const targetAngle = (coordinate - Math.floor(coordinate)) * Math.PI / 2;
  let low = 0;
  let high = 1;
  for (let iteration = 0; iteration < 48; iteration++) {
    const middle = (low + high) / 2;
    const point = quarterPoint(middle);
    const angle = -Math.atan2(point[1], point[0]);
    if (angle < targetAngle) low = middle;
    else high = middle;
  }
  const t = (low + high) / 2;
  let point = quarterPoint(t);
  let tangent = quarterDerivative(t);
  for (let turn = 0; turn < quadrant; turn++) {
    point = [point[1], -point[0]];
    tangent = [tangent[1], -tangent[0]];
  }
  const tangentLength = Math.hypot(...tangent);
  tangent = tangent.map((value) => value / tangentLength);
  return {
    point,
    tangent,
    outward: [-tangent[1], tangent[0]],
  };
}

function circleFrameAtParameter(fraction) {
  const coordinate = fraction * 4;
  const quadrant = Math.floor(coordinate) % 4;
  const t = coordinate - Math.floor(coordinate);
  let point = quarterPoint(t);
  let tangent = quarterDerivative(t);
  for (let turn = 0; turn < quadrant; turn++) {
    point = [point[1], -point[0]];
    tangent = [tangent[1], -tangent[0]];
  }
  const tangentLength = Math.hypot(...tangent);
  tangent = tangent.map((value) => value / tangentLength);
  return {
    point,
    tangent,
    outward: [-tangent[1], tangent[0]],
  };
}

function modeledTailPoints(frame, center, tailRadius, elementSize, boldWidth) {
  const waveAmplitude = boldWidth / 2;
  const segmentCount = Math.max(1, Math.round(elementSize * 1.6));
  const radialStep = 3 * elementSize / (3 * elementSize + 2);
  const waveTangent = frame.tangent.map((value) => -value);
  const baselineRadius = tailRadius - elementSize * 0.5;
  const start = [
    center[0] + frame.point[0] * baselineRadius +
      frame.outward[0] * elementSize * 0.5,
    center[1] + frame.point[1] * baselineRadius +
      frame.outward[1] * elementSize * 0.5,
  ];
  const points = [start];
  let previous = start;
  for (let segment = 0; segment < segmentCount; segment++) {
    const phase = segment % 4;
    const previousTangent = [0, -waveAmplitude, 0, waveAmplitude][phase];
    const components = [
      [-waveAmplitude, 0, -WAVE_CONTROL * waveAmplitude,
        (1 - WAVE_CONTROL) * radialStep, -waveAmplitude],
      [0, WAVE_CONTROL * radialStep, -waveAmplitude, radialStep,
        -WAVE_CONTROL * waveAmplitude],
      [waveAmplitude, 0, WAVE_CONTROL * waveAmplitude,
        (1 - WAVE_CONTROL) * radialStep, waveAmplitude],
      [0, WAVE_CONTROL * radialStep, waveAmplitude, radialStep,
        WAVE_CONTROL * waveAmplitude],
    ][phase];
    const [
      nextTangent,
      control1Radial,
      control1Tangent,
      control2Radial,
      control2Tangent,
    ] = components;
    const translate = (origin, radial, tangent) => [
      origin[0] - frame.outward[0] * radial +
        waveTangent[0] * (tangent - previousTangent),
      origin[1] - frame.outward[1] * radial +
        waveTangent[1] * (tangent - previousTangent),
    ];
    const control1 = translate(
      previous,
      control1Radial,
      control1Tangent,
    );
    const control2 = translate(
      previous,
      control2Radial,
      control2Tangent,
    );
    const end = [
      start[0] - frame.outward[0] * radialStep * (segment + 1) +
        waveTangent[0] * nextTangent,
      start[1] - frame.outward[1] * radialStep * (segment + 1) +
        waveTangent[1] * nextTangent,
    ];
    points.push(control1, control2, end);
    previous = end;
  }
  return points.flat();
}

const manifest = JSON.parse(
  await fs.readFile("tmp/chemdraw-bioshape-geometry-probe/manifest.json", "utf8"),
);

for (const entry of manifest.cases.filter(
  (candidate) => candidate.type === "MembraneMicelle",
)) {
  const svg = await fs.readFile(entry.svg, "utf8");
  const blackPaths = [...svg.matchAll(/<path\b([^>]*)>/gi)]
    .map((match) => match[1])
    .filter((attributes) => /stroke="#000000"/i.test(attributes));
  const circleCenters = blackPaths
    .filter((attributes) => /\bA\b/.test(attributes))
    .map((attributes) => {
      const d = attributes.match(/\bd="([^"]*)"/i)[1];
      const match = d.match(
        /M\s*([-\d.]+),([-\d.]+)\s*A\s*[-\d.]+,[-\d.]+\s+\d+\s+\d+\s+\d+\s*([-\d.]+),([-\d.]+)/,
      );
      return [
        (Number(match[1]) + Number(match[3])) / 40,
        (Number(match[2]) + Number(match[4])) / 40,
      ];
    });
  const center = entry.axes.minor.slice(0, 2);
  const measuredCenter = [
    circleCenters.reduce((sum, point) => sum + point[0], 0) /
      circleCenters.length,
    circleCenters.reduce((sum, point) => sum + point[1], 0) /
      circleCenters.length,
  ];
  const headAngles = [];
  const headRadii = [];
  for (const headCenter of circleCenters) {
    headRadii.push(
      Math.hypot(headCenter[0] - center[0], headCenter[1] - center[1]),
    );
    let angle = Math.atan2(headCenter[1] - center[1], headCenter[0] - center[0]);
    if (headAngles.length > 0) {
      while (angle > headAngles.at(-1)) angle -= 2 * Math.PI;
    }
    headAngles.push(angle);
  }
  const headAngleSteps = headAngles.slice(1).map(
    (angle, index) => angle - headAngles[index],
  );
  let modelHeadMaximumDelta = 0;
  let modelTailStartMaximumDelta = 0;
  let angleModelHeadMaximumDelta = 0;
  let angleModelTailStartMaximumDelta = 0;
  const tailModelMaximumDeltas = {
    arcLength: 0,
    angle: 0,
    parameter: 0,
    observedAngle: 0,
  };
  const normalModelMaximumAngleDeltas = {
    arcLength: 0,
    angle: 0,
    parameter: 0,
    observedAngle: 0,
  };
  const centerlineModelMaximumDeltas = {
    arcLength: 0,
    angle: 0,
    parameter: 0,
    observedAngle: 0,
  };
  let headTailNormalLengthMaximumDelta = 0;
  let headTailVersusTailAxisMaximumAngleDelta = 0;
  const measuredCenterlineRadii = [];
  const measuredCenterlines = [];
  const measuredTailValues = [];
  const measuredNormals = [];
  const headRadius = Math.hypot(
    entry.axes.major[0] - entry.axes.center[0],
    entry.axes.major[1] - entry.axes.center[1],
  ) * 1.2 + entry.requestedParameters.MembraneElementSize;
  const tailRadius = headRadius -
    entry.requestedParameters.MembraneElementSize * 0.5;
  const tailPaths = blackPaths.filter((attributes) =>
    /\bC\b/.test(attributes.match(/\bd="([^"]*)"/i)?.[1] ?? "")
  );
  for (let index = 0; index < circleCenters.length; index++) {
    const frame = circleFrame((index + 1) / circleCenters.length);
    const angleFrame = circleFrameAtAngle(
      (index + 1) / circleCenters.length,
    );
    const parameterFrame = circleFrameAtParameter(
      (index + 1) / circleCenters.length,
    );
    const baselineRadius = headRadius -
      entry.requestedParameters.MembraneElementSize;
    const predictedHead = [
      center[0] + frame.point[0] * baselineRadius +
        frame.outward[0] * entry.requestedParameters.MembraneElementSize,
      center[1] + frame.point[1] * baselineRadius +
        frame.outward[1] * entry.requestedParameters.MembraneElementSize,
    ];
    modelHeadMaximumDelta = Math.max(
      modelHeadMaximumDelta,
      Math.abs(predictedHead[0] - circleCenters[index][0]),
      Math.abs(predictedHead[1] - circleCenters[index][1]),
    );
    const tailValues = [
      ...tailPaths[index].match(/\bd="([^"]*)"/i)[1].matchAll(
        /-?\d+(?:\.\d+)?/g,
      ),
    ].map((match) => Number(match[0]) / 20);
    const headTail = [
      circleCenters[index][0] - tailValues[0],
      circleCenters[index][1] - tailValues[1],
    ];
    const headTailLength = Math.hypot(...headTail);
    measuredNormals.push(headTail.map((value) => value / headTailLength));
    headTailNormalLengthMaximumDelta = Math.max(
      headTailNormalLengthMaximumDelta,
      Math.abs(
        Math.hypot(...headTail) -
          entry.requestedParameters.MembraneElementSize * 0.5,
      ),
    );
    const measuredCenterline = [
      tailValues[0] - headTail[0],
      tailValues[1] - headTail[1],
    ];
    const measuredCenterlineAngle = Math.atan2(
      measuredCenterline[1] - center[1],
      measuredCenterline[0] - center[0],
    );
    const observedAngleFrame = circleFrameAtAngle(
      ((-measuredCenterlineAngle / (2 * Math.PI)) % 1 + 1) % 1,
    );
    if (tailValues.length >= 14) {
      const tailAxis = [
        tailValues[0] - tailValues[12],
        tailValues[1] - tailValues[13],
      ];
      const dot = (
        headTail[0] * tailAxis[0] + headTail[1] * tailAxis[1]
      ) / (Math.hypot(...headTail) * Math.hypot(...tailAxis));
      headTailVersusTailAxisMaximumAngleDelta = Math.max(
        headTailVersusTailAxisMaximumAngleDelta,
        Math.acos(Math.min(1, Math.max(-1, dot))),
      );
    }
    measuredCenterlineRadii.push(
      Math.hypot(
        measuredCenterline[0] - center[0],
        measuredCenterline[1] - center[1],
      ),
    );
    measuredCenterlines.push(measuredCenterline);
    measuredTailValues.push(tailValues);
    const predictedStart = [
      center[0] + frame.point[0] * baselineRadius +
        frame.outward[0] *
          entry.requestedParameters.MembraneElementSize * 0.5,
      center[1] + frame.point[1] * baselineRadius +
        frame.outward[1] *
          entry.requestedParameters.MembraneElementSize * 0.5,
    ];
    modelTailStartMaximumDelta = Math.max(
      modelTailStartMaximumDelta,
      Math.abs(predictedStart[0] - tailValues[0]),
      Math.abs(predictedStart[1] - tailValues[1]),
    );
    const anglePredictedHead = [
      center[0] + angleFrame.point[0] * baselineRadius +
        angleFrame.outward[0] * entry.requestedParameters.MembraneElementSize,
      center[1] + angleFrame.point[1] * baselineRadius +
        angleFrame.outward[1] * entry.requestedParameters.MembraneElementSize,
    ];
    angleModelHeadMaximumDelta = Math.max(
      angleModelHeadMaximumDelta,
      Math.abs(anglePredictedHead[0] - circleCenters[index][0]),
      Math.abs(anglePredictedHead[1] - circleCenters[index][1]),
    );
    const anglePredictedStart = [
      center[0] + angleFrame.point[0] * baselineRadius +
        angleFrame.outward[0] *
          entry.requestedParameters.MembraneElementSize * 0.5,
      center[1] + angleFrame.point[1] * baselineRadius +
        angleFrame.outward[1] *
          entry.requestedParameters.MembraneElementSize * 0.5,
    ];
    angleModelTailStartMaximumDelta = Math.max(
      angleModelTailStartMaximumDelta,
      Math.abs(anglePredictedStart[0] - tailValues[0]),
      Math.abs(anglePredictedStart[1] - tailValues[1]),
    );
    for (const [model, candidateFrame] of Object.entries({
      arcLength: frame,
      angle: angleFrame,
      parameter: parameterFrame,
      observedAngle: observedAngleFrame,
    })) {
      const normalDot = candidateFrame.outward[0] *
          measuredNormals[index][0] +
        candidateFrame.outward[1] * measuredNormals[index][1];
      normalModelMaximumAngleDeltas[model] = Math.max(
        normalModelMaximumAngleDeltas[model],
        Math.acos(Math.min(1, Math.max(-1, normalDot))),
      );
      const modeledCenterline = [
        center[0] + candidateFrame.point[0] * baselineRadius,
        center[1] + candidateFrame.point[1] * baselineRadius,
      ];
      centerlineModelMaximumDeltas[model] = Math.max(
        centerlineModelMaximumDeltas[model],
        Math.abs(modeledCenterline[0] - measuredCenterline[0]),
        Math.abs(modeledCenterline[1] - measuredCenterline[1]),
      );
      const modeled = modeledTailPoints(
        candidateFrame,
        center,
        tailRadius,
        entry.requestedParameters.MembraneElementSize,
        2,
      );
      for (let coordinate = 0; coordinate < tailValues.length; coordinate++) {
        tailModelMaximumDeltas[model] = Math.max(
          tailModelMaximumDeltas[model],
          Math.abs(modeled[coordinate] - tailValues[coordinate]),
        );
      }
    }
  }
  let centerlineGridMaximumDelta = 0;
  let finiteDifferenceNormalMaximumAngleDelta = 0;
  const finiteDifferenceSpanMaximumAngleDeltas = {};
  let uniformNormalMaximumAngleDelta = 0;
  for (let index = 0; index < measuredCenterlines.length; index++) {
    for (const value of measuredCenterlines[index]) {
      centerlineGridMaximumDelta = Math.max(
        centerlineGridMaximumDelta,
        Math.abs(value * 64 - Math.round(value * 64)) / 64,
      );
    }
    const previous = measuredCenterlines[
      (index + measuredCenterlines.length - 1) % measuredCenterlines.length
    ];
    const next = measuredCenterlines[(index + 1) % measuredCenterlines.length];
    const tangent = [next[0] - previous[0], next[1] - previous[1]];
    const tangentLength = Math.hypot(...tangent);
    const finiteNormal = [-tangent[1] / tangentLength, tangent[0] / tangentLength];
    const tailValues = measuredTailValues[index];
    const tailAxis = [
      tailValues[0] - tailValues[12],
      tailValues[1] - tailValues[13],
    ];
    const tailAxisLength = Math.hypot(...tailAxis);
    const observedNormal = tailAxis.map((value) => value / tailAxisLength);
    const angle = -2 * Math.PI * (index + 1) / measuredCenterlines.length;
    const uniformNormal = [Math.cos(angle), Math.sin(angle)];
    uniformNormalMaximumAngleDelta = Math.max(
      uniformNormalMaximumAngleDelta,
      Math.acos(Math.min(1, Math.max(-1,
        uniformNormal[0] * measuredNormals[index][0] +
        uniformNormal[1] * measuredNormals[index][1]))),
    );
    finiteDifferenceNormalMaximumAngleDelta = Math.max(
      finiteDifferenceNormalMaximumAngleDelta,
      Math.abs(
        Math.acos(Math.min(1, Math.max(-1,
          finiteNormal[0] * observedNormal[0] +
          finiteNormal[1] * observedNormal[1]))),
      ),
    );
    for (let span = 1; span <= 8; span++) {
      const spanPrevious = measuredCenterlines[
        (index + measuredCenterlines.length - span) %
          measuredCenterlines.length
      ];
      const spanNext = measuredCenterlines[
        (index + span) % measuredCenterlines.length
      ];
      const spanTangent = [
        spanNext[0] - spanPrevious[0],
        spanNext[1] - spanPrevious[1],
      ];
      const spanLength = Math.hypot(...spanTangent);
      const spanNormal = [
        -spanTangent[1] / spanLength,
        spanTangent[0] / spanLength,
      ];
      const angleDelta = Math.acos(Math.min(1, Math.max(-1,
        spanNormal[0] * observedNormal[0] +
        spanNormal[1] * observedNormal[1])));
      finiteDifferenceSpanMaximumAngleDeltas[span] = Math.max(
        finiteDifferenceSpanMaximumAngleDeltas[span] ?? 0,
        angleDelta,
      );
    }
  }
  const quantizedNormalFits = [];
  for (const mode of ["round", "floor", "ceil", "trunc"]) {
    let best = null;
    for (let scale = 8; scale <= 256; scale += 0.125) {
      let maximum = 0;
      let squared = 0;
      for (let index = 0; index < measuredNormals.length; index++) {
        const angle = -2 * Math.PI * (index + 1) / measuredNormals.length;
        const quantize = Math[mode];
        const x = quantize(scale * Math.cos(angle));
        const y = quantize(scale * Math.sin(angle));
        const length = Math.hypot(x, y);
        const dot = (x * measuredNormals[index][0] +
          y * measuredNormals[index][1]) / length;
        const delta = Math.acos(Math.min(1, Math.max(-1, dot)));
        maximum = Math.max(maximum, delta);
        squared += delta * delta;
      }
      const rms = Math.sqrt(squared / measuredNormals.length);
      if (best == null || rms < best.rms) best = { mode, scale, maximum, rms };
    }
    quantizedNormalFits.push(best);
  }
  const tail = blackPaths.find((attributes) =>
    /\bC\b/.test(attributes.match(/\bd="([^"]*)"/i)?.[1] ?? "")
  );
  const tailAngles = blackPaths
    .filter((attributes) =>
      /\bC\b/.test(attributes.match(/\bd="([^"]*)"/i)?.[1] ?? "")
    )
    .map((attributes) => {
      const values = [
        ...attributes.match(/\bd="([^"]*)"/i)[1].matchAll(
          /-?\d+(?:\.\d+)?/g,
        ),
      ].map((match) => Number(match[0]) / 20);
      return Math.atan2(values[1] - values[13], values[0] - values[12]);
    });
  const unwrappedAngles = [];
  for (const angle of tailAngles) {
    let unwrapped = angle;
    if (unwrappedAngles.length > 0) {
      while (unwrapped > unwrappedAngles.at(-1)) unwrapped -= 2 * Math.PI;
    }
    unwrappedAngles.push(unwrapped);
  }
  const angleSteps = unwrappedAngles.slice(1).map(
    (angle, index) => angle - unwrappedAngles[index],
  );
  const d = tail.match(/\bd="([^"]*)"/i)[1];
  const values = [...d.matchAll(/-?\d+(?:\.\d+)?/g)].map(
    (match) => Number(match[0]) / 20,
  );
  const points = Array.from({ length: values.length / 2 }, (_, index) => [
    values[index * 2],
    values[index * 2 + 1],
  ]);
  const start = points[0];
  const end = points.at(-1);
  const head = circleCenters[0];
  const radialLength = Math.hypot(head[0] - center[0], head[1] - center[1]);
  const outward = [
    (head[0] - center[0]) / radialLength,
    (head[1] - center[1]) / radialLength,
  ];
  const tangent = [-outward[1], outward[0]];
  const delta = [end[0] - start[0], end[1] - start[1]];
  console.log(JSON.stringify({
    variant: entry.variant,
    elementSize: entry.requestedParameters.MembraneElementSize,
    cubicCount: (d.match(/\bC\b/g) ?? []).length,
    chordLength: Math.hypot(end[0] - start[0], end[1] - start[1]),
    radialAdvance: -(delta[0] * outward[0] + delta[1] * outward[1]),
    tangentAdvance: delta[0] * tangent[0] + delta[1] * tangent[1],
    angleFirst: unwrappedAngles[0],
    angleLast: unwrappedAngles.at(-1),
    angleStepMean: angleSteps.reduce((sum, step) => sum + step, 0) /
      angleSteps.length,
    angleStepRange: [Math.min(...angleSteps), Math.max(...angleSteps)],
    headAngleFirst: headAngles[0],
    headAngleLast: headAngles.at(-1),
    headAngleStepMean: headAngleSteps.reduce((sum, step) => sum + step, 0) /
      headAngleSteps.length,
    headAngleStepRange: [
      Math.min(...headAngleSteps),
      Math.max(...headAngleSteps),
    ],
    headRadiusRange: [Math.min(...headRadii), Math.max(...headRadii)],
    measuredCenter,
    modelHeadMaximumDelta,
    modelTailStartMaximumDelta,
    angleModelHeadMaximumDelta,
    angleModelTailStartMaximumDelta,
    tailModelMaximumDeltas,
    normalModelMaximumAngleDeltas,
    centerlineModelMaximumDeltas,
    headTailNormalLengthMaximumDelta,
    headTailVersusTailAxisMaximumAngleDelta,
    measuredCenterlineRadiusRange: [
      Math.min(...measuredCenterlineRadii),
      Math.max(...measuredCenterlineRadii),
    ],
    centerlineGridMaximumDelta,
    finiteDifferenceNormalMaximumAngleDelta,
    finiteDifferenceSpanMaximumAngleDeltas,
    uniformNormalMaximumAngleDelta,
    quantizedNormalFits,
    normalSamples: [0, Math.floor(circleCenters.length / 8),
      Math.floor(circleCenters.length / 4)].map((index) => ({
      index,
      measured: measuredNormals[index],
      uniform: [
        Math.cos(-2 * Math.PI * (index + 1) / circleCenters.length),
        Math.sin(-2 * Math.PI * (index + 1) / circleCenters.length),
      ],
      scaledByBaseline: measuredNormals[index].map((value) =>
        value * (headRadius - entry.requestedParameters.MembraneElementSize)
      ),
      centerline: measuredCenterlines[index],
    })),
    normalCorrectionSamples: Array.from(
      { length: 17 },
      (_, sample) => Math.min(
        measuredNormals.length - 1,
        Math.floor(sample * measuredNormals.length / 16),
      ),
    ).map((index) => {
      const uniformAngle = -2 * Math.PI * (index + 1) /
        measuredNormals.length;
      const measuredAngle = Math.atan2(
        measuredNormals[index][1],
        measuredNormals[index][0],
      );
      let delta = measuredAngle - uniformAngle;
      while (delta > Math.PI) delta -= 2 * Math.PI;
      while (delta < -Math.PI) delta += 2 * Math.PI;
      return [index, uniformAngle, delta *
        (headRadius - entry.requestedParameters.MembraneElementSize)];
    }),
    points,
  }));
}
