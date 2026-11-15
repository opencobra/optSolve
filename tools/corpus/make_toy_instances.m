function summary = make_toy_instances(options)
% make_toy_instances  Build, write, cross-check and record the small toy test instances.
%
% Usage
%   summary = make_toy_instances()
%   summary = make_toy_instances(CheckOnly = true)
%
% Gurobi's and MOSEK's MATLAB interfaces (gurobi, gurobi_read, gurobi_write, mosekopt)
% must be on the MATLAB path with their licences available.
%
% Every toy instance is defined in localDefinitions below, in the solver-independent
% notation of the constitution: minimise (sigma = +1) or maximise (sigma = -1)
% 1/2 x'Qx + c'x + gamma subject to b_l <= Ax <= b_u, l <= x <= u and Fx + g in K, with
% the cones of K on consecutive rows of F. Each definition carries its analytic optimum
% and the closed-form argument (derivation). For every definition this function
%   1. writes the file with the tool that owns the format: Gurobi (gurobi_write) for MPS
%      (LP, QP), MOSEK (mosekopt write) for JTASK (conic);
%   2. reads the file back with the owning tool and requires the data to equal the
%      definition exactly (and, for MPS, MOSEK's reading of the file too); for MPS it also
%      requires every column to appear in COLUMNS and the QUADOBJ entries to equal the
%      upper triangle of Q;
%   3. solves the file's read-back problem with MOSEK, and with Gurobi from the file (LP,
%      QP) or from a Gurobi model built from the definition (conic instances whose cones
%      Gurobi supports: quadratic, rotated quadratic, nonnegative, zero);
%   4. requires every status, and every Optimal objective within
%      1e-6 * (1 + |expected objective|), to match the analytic value, and stops otherwise;
%   5. unless CheckOnly, copies the file to the small fixture folder and writes
%      <name>.provenance.json.
% The four pre-existing small LP fixtures (infeasible, unbounded, ranged, empty_column)
% are read with both tools (their readings must describe the same problem), solved with
% both, and gain an independent_checks field in their provenance records; no other field
% and no MPS file is changed.
%
% Gurobi is run with DualReductions = 0 (so infeasible and unbounded are told apart) and
% BarQCPConvTol = 1e-9; MOSEK with its defaults.
%
% Name-value options
%   CheckOnly   (false) run every check and write nothing under the fixture folder.
%   RunDir      ('') folder for the log and scratch files; default
%               <generated>/<Feature>/runs/<UTC>-toy, resolved through project_params.py.
%   Feature     ('003-llm-interface-toy-instances') name of the generated-output home.
%
% Output
%   summary   table, one row per (instance, tool): status, objective, expected values.
%
% Errors carry an identifier. Nothing is written for an instance whose checks fail.

arguments
    options.CheckOnly (1, 1) logical = false
    options.RunDir {mustBeTextScalar} = ''
    options.Feature {mustBeTextScalar} = '003-llm-interface-toy-instances'
end

repoRoot = localRepoRoot();
scriptRelPath = 'tools/corpus/make_toy_instances.m';
licenceText = 'MIT OR Apache-2.0, project-authored';

[status, out] = system(sprintf('python3 -I "%s" --json', ...
    fullfile(repoRoot, '.specify', 'scripts', 'python', 'project_params.py')));
if status ~= 0
    error('make_toy_instances:params', 'project_params.py failed: %s', out);
end
params = jsondecode(out);
fixtureDir = fullfile(repoRoot, params.test_corpus.fixtures, 'small');

runDir = char(options.RunDir);
if isempty(runDir)
    stamp = char(datetime('now', 'TimeZone', 'UTC', 'Format', 'yyyyMMdd''T''HHmmss''Z'''));
    runDir = fullfile(repoRoot, params.structure.generated, char(options.Feature), ...
        'runs', [stamp '-toy']);
end
if ~isfolder(runDir)
    mkdir(runDir);
end
scratchDir = fullfile(runDir, 'files');
if ~isfolder(scratchDir)
    mkdir(scratchDir);
end
diary(fullfile(runDir, 'log.txt'));
stopDiary = onCleanup(@() diary('off'));
fprintf('make_toy_instances: run folder %s (CheckOnly = %d)\n', runDir, options.CheckOnly);

if isempty(which('mosekopt')) || isempty(which('gurobi'))
    error('make_toy_instances:path', 'mosekopt and gurobi must be on the MATLAB path.');
end
optSolverCommit = localGit(repoRoot);
[~, dirty] = system(sprintf('git -C "%s" status --porcelain', repoRoot));
optSolverClean = isempty(strtrim(dirty));
tools = localToolVersions();
fprintf('MOSEK %s, Gurobi %s, optSolver commit %s (worktree clean: %d)\n', ...
    tools.mosek, tools.gurobi, optSolverCommit, optSolverClean);
[~, symbols] = mosekopt('symbcon echo(0)');
sym = symbols.symbcon;

defs = localDefinitions();
rows = {};
for i = 1:numel(defs)
    d = defs(i);
    fprintf('\n[%s] %s, %s, %s\n', d.name, d.class, d.sense, d.format);
    file = fullfile(scratchDir, [d.name '.' d.format]);
    if strcmp(d.format, 'mps')
        localWriteMps(d, file);
        checks = localCheckMpsFile(d, file);
    else
        prob = localMosekProb(d, sym);
        [rcode, ~] = mosekopt(sprintf('%s write(%s) echo(0)', localMosekSense(d.sense), file), prob);
        localRequireOk(d.name, 'write', rcode);
        checks = localCheckJtaskFile(d, file, prob);
    end
    for j = 1:numel(checks)
        localRequireAgreement(d.name, checks{j}, d.expected_status, d.expected_objective);
        localRequireX(d.name, checks{j}, d.expected_x);
        rows{end + 1} = localRow(d.name, checks{j}, d.expected_status, ...
            d.expected_objective); %#ok<AGROW>
        checks{j} = rmfield(checks{j}, 'x');
    end
    if options.CheckOnly
        continue
    end
    target = fullfile(fixtureDir, [d.name '.' d.format]);
    copyfile(file, target);
    rec = localRecord(d, target, checks, scriptRelPath, optSolverCommit, ...
        optSolverClean, licenceText);
    localWriteText(fullfile(fixtureDir, [d.name '.provenance.json']), ...
        [jsonencode(rec, 'PrettyPrint', true) newline]);
    fprintf('[%s] wrote %s and its provenance record\n', d.name, target);
end

% The pre-existing small LP fixtures: read with both tools, solve, add the checks.
existing = {'infeasible', 'unbounded', 'ranged', 'empty_column'};
for i = 1:numel(existing)
    name = existing{i};
    file = fullfile(fixtureDir, [name '.mps']);
    recFile = fullfile(fixtureDir, [name '.provenance.json']);
    recText = fileread(recFile);
    rec = jsondecode(recText);
    fprintf('\n[%s] existing fixture\n', name);
    if ~strcmp(localSha256(file), rec.mps_sha256)
        error('make_toy_instances:hash', '[%s] MPS file hash differs from its record.', name);
    end
    expectedObjective = rec.expected_objective;
    if isempty(expectedObjective)
        expectedObjective = NaN;
    end
    checks = localCheckExistingMps(name, file);
    for j = 1:numel(checks)
        localRequireAgreement(name, checks{j}, rec.expected_status, expectedObjective);
        rows{end + 1} = localRow(name, checks{j}, rec.expected_status, ...
            expectedObjective); %#ok<AGROW>
        checks{j} = rmfield(checks{j}, 'x');
    end
    if options.CheckOnly
        continue
    end
    localWriteText(recFile, localAppendChecks(recText, checks));
    if ~strcmp(localSha256(file), rec.mps_sha256)
        error('make_toy_instances:hash', '[%s] MPS file changed.', name);
    end
    fprintf('[%s] added independent_checks to %s\n', name, recFile);
end
summary = struct2table([rows{:}]);
fprintf('\n');
disp(summary);
fprintf('make_toy_instances: all %d checks agree with the analytic values\n', height(summary));
end

% =========================================================================
% Definitions
% =========================================================================
function defs = localDefinitions()
% Every toy instance: data in the solver-independent notation, outcome and derivation.
r2 = sqrt(2);
defs = [ ...
    localLp('toy_lp_max', 'max', [1; 2], sparse([1 1; 1 3]), [-inf; -inf], [4; 6], ...
        [0; 0], [inf; inf], 5, [3; 1], ...
        ['Maximise x1 + 2 x2 subject to x1 + x2 <= 4, x1 + 3 x2 <= 6, x >= 0. The ' ...
        'feasible polygon has vertices (0, 0), (4, 0), (0, 2) and (3, 1) (the two ' ...
        'rows active: subtracting gives 2 x2 = 2). Objective values 0, 4, 4, 5: the ' ...
        'maximum 5 is attained only at (3, 1), since every other vertex is strictly ' ...
        'smaller.']), ...
    localLp('toy_lp_mixed', 'min', [2; 3; 1], sparse([1 1 1; 1 -1 0; 0 1 1]), ...
        [4; 1; -inf], [4; inf; 3], [0; 0; 0], [10; 10; 10], 5, [1; 0; 3], ...
        ['Minimise 2 x1 + 3 x2 + x3 subject to x1 + x2 + x3 = 4, x1 - x2 >= 1, ' ...
        'x2 + x3 <= 3, 0 <= x <= 10. Eliminating x3 = 4 - x1 - x2 gives objective ' ...
        '4 + x1 + 2 x2 with x1 - x2 >= 1, x1 >= 1 (from x2 + x3 <= 3) and ' ...
        'x1 + x2 <= 4 (from x3 >= 0). Since x1 >= 1 and x2 >= 0 with positive ' ...
        'coefficients, the minimum 4 + 1 + 0 = 5 is attained only at x1 = 1, x2 = 0, ' ...
        'hence x3 = 3.']), ...
    localQp('toy_qp_diag', 'min', speye(2), [0; 0], 1, [1; 1], ...
        ['Minimise 1/2 (x1^2 + x2^2) subject to x1 + x2 = 2, x >= 0. The objective ' ...
        'is strictly convex, so the optimum is unique; stationarity of the ' ...
        'Lagrangian gives x1 = x2, hence x = (1, 1) and objective 1/2 (1 + 1) = 1 ' ...
        '(the bounds are inactive).']), ...
    localQp('toy_qp_offdiag', 'min', sparse([2 1; 1 2]), [0; 0], 3, [1; 1], ...
        ['Q = [2 1; 1 2] (positive definite, eigenvalues 1 and 3), c = 0: minimise ' ...
        'x1^2 + x1 x2 + x2^2 subject to x1 + x2 = 2, x >= 0. Stationarity: ' ...
        '2 x1 + x2 = x1 + 2 x2, so x1 = x2 = 1, unique by strict convexity; ' ...
        'objective 1 + 1 + 1 = 3.']), ...
    localQp('toy_qp_max', 'max', -speye(2), [-1; 0], -7/4, [1/2; 3/2], ...
        ['Maximise -1/2 (x1^2 + x2^2) - x1 subject to x1 + x2 = 2, x >= 0 (Q = -I, ' ...
        'c = (-1, 0); the objective is strictly concave, so the optimum is unique). ' ...
        'Substituting x2 = 2 - x1: -1/2 x1^2 - 1/2 (2 - x1)^2 - x1, derivative ' ...
        '-2 x1 + 1 = 0, so x1 = 1/2, x2 = 3/2 (bounds inactive); objective ' ...
        '-1/2 (1/4 + 9/4) - 1/2 = -7/4.'])];

% Conic instances. Variables are 0-based x0, x1, ... in the prose; F = diag(f).
defs = [defs, ...
    localClp('toy_clp_quad', [1; 0; 0], {localCone('quadratic', 3)}, [1; 1; 1], ...
        [0; 0; 0], [1 3; 2 4], inf, 'Optimal', 5, [5; 3; 4], ...
        ['Quadratic cone of dimension 3 on (x0, x1, x2): x0 >= sqrt(x1^2 + x2^2). ' ...
        'With x1 = 3 and x2 = 4 fixed, minimising x0 gives x0 = sqrt(9 + 16) = 5, ' ...
        'unique; objective 5.']), ...
    localClp('toy_clp_rquad', [1; 0; 0], {localCone('rotated quadratic', 3)}, ...
        [1; 1; 1], [0; 0; 0], [1 1; 2 2], inf, 'Optimal', 2, [2; 1; 2], ...
        ['Rotated quadratic cone of dimension 3 on (x0, x1, x2): 2 x0 x1 >= x2^2, ' ...
        'x0, x1 >= 0. With x1 = 1 and x2 = 2 fixed, 2 x0 >= 4, so the minimum of x0 ' ...
        'is 2, unique; objective 2.']), ...
    localClp('toy_clp_pexp', [1; 0; 0], {localCone('primal exponential', 3)}, ...
        [1; 1; 1], [0; 0; 0], [1 1; 2 1], inf, 'Optimal', exp(1), [exp(1); 1; 1], ...
        ['Primal exponential cone on (x0, x1, x2): x0 >= x1 exp(x2 / x1), x1 > 0. ' ...
        'With x1 = 1 and x2 = 1 fixed, x0 >= exp(1), so the minimum is e, attained ' ...
        'only at x0 = e; objective e.']), ...
    localClp('toy_clp_dexp', [1; 0; 0], {localCone('dual exponential', 3)}, ...
        [1; 1; 1], [0; 0; 0], [1 0; 2 -1], inf, 'Optimal', exp(-1), ...
        [exp(-1); 0; -1], ...
        ['Dual exponential cone on (x0, x1, x2): x0 >= -x2 exp(x1 / x2 - 1), ' ...
        'x0 > 0, x2 < 0. With x1 = 0 and x2 = -1 fixed, x0 >= 1 * exp(0 - 1) = 1/e, ' ...
        'so the minimum is 1/e, attained only at x0 = 1/e; objective 1/e.']), ...
    localClp('toy_clp_ppow', [1; 0; 0], {localCone('primal power', 3, [1 1])}, ...
        [1; 1; 1], [0; 0; 0], [1 4; 2 2], inf, 'Optimal', 1, [1; 4; 2], ...
        ['Primal power cone of dimension 3 with exponents (1, 1), normalised to ' ...
        '(1/2, 1/2), on (x0, x1, x2): x0^(1/2) x1^(1/2) >= |x2|, x0, x1 >= 0. With ' ...
        'x1 = 4 and x2 = 2 fixed, sqrt(4 x0) >= 2, so x0 >= 1; the minimum 1 is ' ...
        'unique; objective 1.']), ...
    localClp('toy_clp_dpow', [1; 0; 0], {localCone('dual power', 3, [1 1])}, ...
        [1; 1; 1], [0; 0; 0], [1 1; 2 4], inf, 'Optimal', 4, [4; 1; 4], ...
        ['Dual power cone of dimension 3 with exponents (1, 1), normalised to ' ...
        'alpha = (1/2, 1/2), on (x0, x1, x2): (x0 / alpha1)^alpha1 (x1 / alpha2)^alpha2 ' ...
        '>= |x2|, i.e. 2 sqrt(x0 x1) >= |x2|, x0, x1 >= 0. With x1 = 1 and x2 = 4 ' ...
        'fixed, 2 sqrt(x0) >= 4, so x0 >= 4; the minimum 4 is unique; objective 4.']), ...
    localClp('toy_clp_rplus', [1; 1], {localCone('nonnegative', 2)}, [1; 1], ...
        [0; 0], [1 2], inf, 'Optimal', 2, [0; 2], ...
        ['Nonnegative cone of dimension 2 on (x0, x1): x0, x1 >= 0. With x1 = 2 ' ...
        'fixed, minimising x0 + x1 gives x0 = 0 (its lower limit), unique; ' ...
        'objective 0 + 2 = 2.']), ...
    localClp('toy_clp_rzero', [1; 1], {localCone('zero', 2)}, [1; 1], [-1; -2], ...
        zeros(0, 2), inf, 'Optimal', 3, [1; 2], ...
        ['Zero cone of dimension 2: x + g = 0 with g = (-1, -2), so x = (1, 2) is ' ...
        'the only feasible point (no linear rows); objective 1 + 2 = 3.']), ...
    localClp('toy_clp_psd2', [1; 0; 1], {localCone('positive semidefinite', 3)}, ...
        [1; r2; 1], [0; 0; 0], [1 1], inf, 'Optimal', 2, [1; 1; 1], ...
        ['Positive semidefinite cone of order 2 in svec form (lower triangle column ' ...
        'by column, off-diagonal scaled by sqrt(2)): F x = (x0, sqrt(2) x1, x2) is ' ...
        'svec of X = [x0 x1; x1 x2]. With x1 = 1 fixed, X is PSD iff x0, x2 >= 0 and ' ...
        'x0 x2 >= 1; minimising x0 + x2 = trace(X): by the arithmetic-geometric mean ' ...
        'inequality x0 + x2 >= 2 sqrt(x0 x2) >= 2, with equality only at ' ...
        'x0 = x2 = 1; objective 2.']), ...
    localClp('toy_clp_psd3', [1; 0; 0; 1; 0; 1], ...
        {localCone('positive semidefinite', 6)}, [1; r2; r2; 1; r2; 1], zeros(6, 1), ...
        [1 1; 2 0; 4 0], inf, 'Optimal', 2, [1; 1; 0; 1; 0; 0], ...
        ['Positive semidefinite cone of order 3 in svec form: F x = (x0, sqrt(2) x1, ' ...
        'sqrt(2) x2, x3, sqrt(2) x4, x5) is svec of X = [x0 x1 x2; x1 x3 x4; ' ...
        'x2 x4 x5] (lower triangle column by column). With x1 = 1, x2 = 0, x4 = 0 ' ...
        'fixed, X = blkdiag([x0 1; 1 x3], x5) is PSD iff x5 >= 0, x0, x3 >= 0 and ' ...
        'x0 x3 >= 1. Minimising trace(X) = x0 + x3 + x5: x5 = 0 and, by the ' ...
        'arithmetic-geometric mean inequality, x0 + x3 >= 2 with equality only at ' ...
        'x0 = x3 = 1; objective 2.']), ...
    localClp('toy_clp_infeasible', [1; 0; 0], {localCone('quadratic', 3)}, ...
        [1; 1; 1], [0; 0; 0], [1 3; 2 4], 4, 'Infeasible', NaN, NaN, ...
        ['Quadratic cone of dimension 3 on (x0, x1, x2) with x1 = 3 and x2 = 4 ' ...
        'fixed requires x0 >= 5, contradicting the upper bound x0 <= 4: no feasible ' ...
        'point.'])];
end

function d = localBase(name, class, format, sense)
d = struct('name', name, 'class', class, 'format', format, 'sense', sense, ...
    'c', [], 'Q', [], 'A', [], 'b_l', [], 'b_u', [], 'l', [], 'u', [], ...
    'F', [], 'g', [], 'cones', {{}}, 'expected_status', 'Optimal', ...
    'expected_objective', NaN, 'expected_x', NaN, 'derivation', '', 'construction', '');
end

function d = localLp(name, sense, c, A, b_l, b_u, l, u, p, x, derivation)
d = localBase(name, 'LP', 'mps', sense);
n = numel(c);
d.c = c;
d.Q = sparse(n, n);
d.A = A;
d.b_l = b_l;
d.b_u = b_u;
d.l = l;
d.u = u;
d.expected_objective = p;
d.expected_x = x;
d.derivation = derivation;
d.construction = ['Defined in tools/corpus/make_toy_instances.m and written by Gurobi ' ...
    '(gurobi_write, MPS)' localSenseNote(sense) '; read back by gurobi_read and by ' ...
    'MOSEK (mosekopt read), both equal to the definition.'];
end

function d = localQp(name, sense, Q, c, p, x, derivation)
% The three toy QPs share the row x1 + x2 = 2 and the bounds x >= 0.
d = localLp(name, sense, c, sparse([1 1]), 2, 2, [0; 0], [inf; inf], p, x, derivation);
d.class = 'QP';
d.Q = sparse(Q);
d.construction = [d.construction ' Gurobi model.Q = Q/2 (Gurobi''s objective is ' ...
    'x''Qx); the QUADOBJ section holds the upper triangle of Q (objective 1/2 x''Qx).'];
end

function d = localClp(name, c, cones, f, g, fixes, uBound0, status, p, x, derivation)
% Conic instance: F = diag(f), equality rows x_i = v from fixes = [i v] (0-based i).
d = localBase(name, 'Conic', 'jtask', 'min');
n = numel(c);
d.c = c;
d.Q = sparse(n, n);
m = size(fixes, 1);
d.A = sparse(1:m, fixes(:, 1) + 1, 1, m, n);
d.b_l = fixes(:, 2);
d.b_u = fixes(:, 2);
d.l = -inf(n, 1);
d.u = inf(n, 1);
d.u(1) = uBound0;
d.F = sparse(diag(f));
d.g = g;
d.cones = cones;
d.expected_status = status;
d.expected_objective = p;
d.expected_x = x;
d.derivation = derivation;
d.construction = ['Defined in tools/corpus/make_toy_instances.m and written by MOSEK ' ...
    '(mosekopt write, JTASK) from prob.a, c, blc, buc, blx, bux, f, g, accs; read back ' ...
    'by mosekopt read, bit-for-bit equal to the written arrays.'];
end

function k = localCone(kind, dimension, alpha)
if nargin < 3
    alpha = NaN;
end
k = struct('kind', kind, 'dimension', dimension, 'alpha', alpha);
end

function s = localSenseNote(sense)
if strcmp(sense, 'max')
    s = [', then its one-line "OBJSENSE MAX" rewritten as the two-line form ' ...
        '"OBJSENSE" / "    MAX" (MOSEK 11.2 rejects the one-line form with error ' ...
        '1100 at the next line)'];
else
    s = '';
end
end

% =========================================================================
% MPS: write, read back, solve
% =========================================================================
function model = localGurobiModel(d)
% The definition as a Gurobi MATLAB model (rows split into =, <, >).
n = numel(d.c);
[A, rhs, sense] = localRowsToSense(d.A, d.b_l, d.b_u);
model = struct('A', A, 'obj', d.c, 'rhs', rhs, 'sense', sense, 'lb', d.l, 'ub', d.u, ...
    'modelsense', d.sense, 'modelname', d.name);
model.varnames = arrayfun(@(j) sprintf('x%d', j), (1:n)', 'UniformOutput', false);
model.constrnames = arrayfun(@(i) sprintf('r%d', i), (1:size(A, 1))', 'UniformOutput', false);
if nnz(d.Q) > 0
    model.Q = d.Q / 2;
end
end

function [A, rhs, sense] = localRowsToSense(A, b_l, b_u)
% Gurobi rows from bounds b_l <= Ax <= b_u; a doubly bounded unequal row is refused.
m = size(A, 1);
rhs = zeros(m, 1);
sense = blanks(m)';
for i = 1:m
    if b_l(i) == b_u(i)
        sense(i) = '=';
        rhs(i) = b_l(i);
    elseif isinf(b_l(i))
        sense(i) = '<';
        rhs(i) = b_u(i);
    elseif isinf(b_u(i))
        sense(i) = '>';
        rhs(i) = b_l(i);
    else
        error('make_toy_instances:ranged', 'Ranged rows are not used in toy definitions.');
    end
end
end

function localWriteMps(d, file)
gurobi_write(localGurobiModel(d), file);
if strcmp(d.sense, 'max')
    text = fileread(file);
    oneLine = sprintf('OBJSENSE MAX\n');
    if numel(strfind(text, oneLine)) ~= 1
        error('make_toy_instances:objsense', '[%s] expected one "OBJSENSE MAX" line.', d.name);
    end
    localWriteText(file, strrep(text, oneLine, sprintf('OBJSENSE\n    MAX\n')));
end
end

function checks = localCheckMpsFile(d, file)
% Read back with both tools, compare to the definition, structural checks, solve.
text = fileread(file);
n = numel(d.c);
localRequireColumnsListed(d.name, text, n);
localRequireQuadobj(d.name, text, d.Q);
want = localCanonical(d.A, d.c, d.Q, d.b_l, d.b_u, d.l, d.u, d.sense, 0);
fromGurobi = localCanonicalGurobi(gurobi_read(file));
fromMosek = localCanonicalMosek(localMosekRead(d.name, file));
localRequireSame(d.name, 'gurobi_read against the definition', want, fromGurobi);
localRequireSame(d.name, 'MOSEK read against the definition', want, fromMosek);
fprintf('[%s] read back by gurobi_read and MOSEK: equal to the definition\n', d.name);
checks = {localSolveMosekFile(file), localSolveGurobiFile(file)};
end

function checks = localCheckExistingMps(name, file)
% Both readings must describe the same problem; then both tools solve the file.
fromGurobi = localCanonicalGurobi(gurobi_read(file));
fromMosek = localCanonicalMosek(localMosekRead(name, file));
localRequireSame(name, 'gurobi_read against MOSEK read', fromMosek, fromGurobi);
fprintf('[%s] gurobi_read and MOSEK read describe the same problem (n = %d, m = %d)\n', ...
    name, numel(fromMosek.c), numel(fromMosek.b_l));
localRequireColumnsListed(name, fileread(file), numel(fromMosek.c));
checks = {localSolveMosekFile(file), localSolveGurobiFile(file)};
end

function localRequireColumnsListed(name, text, n)
% Every column must appear in COLUMNS (a column declared only in BOUNDS is not portable).
section = regexp(text, '(?m)^COLUMNS\s*$(.*?)^RHS', 'tokens', 'once');
if isempty(section)
    error('make_toy_instances:columns', '[%s] no COLUMNS section.', name);
end
lines = strtrim(splitlines(section{1}));
lines = lines(~cellfun(@isempty, lines));
names = unique(cellfun(@(s) strtok(s), lines, 'UniformOutput', false));
if numel(names) ~= n
    error('make_toy_instances:columns', '[%s] COLUMNS lists %d columns, the problem has %d.', ...
        name, numel(names), n);
end
end

function localRequireQuadobj(name, text, Q)
% The QUADOBJ entries must be the upper triangle of Q (names x<j>), nothing else.
section = regexp(text, '(?m)^QUADOBJ\s*$(.*?)^ENDATA', 'tokens', 'once');
if nnz(Q) == 0
    if ~isempty(section)
        error('make_toy_instances:quadobj', '[%s] unexpected QUADOBJ section.', name);
    end
    return
end
lines = strtrim(splitlines(section{1}));
lines = lines(~cellfun(@isempty, lines));
n = size(Q, 1);
written = sparse(n, n);
for i = 1:numel(lines)
    parts = strsplit(lines{i});
    r = sscanf(parts{1}, 'x%d');
    c = sscanf(parts{2}, 'x%d');
    written(r, c) = written(r, c) + str2double(parts{3}); %#ok<SPRIX>
end
if ~isequal(full(written), full(triu(Q)))
    error('make_toy_instances:quadobj', '[%s] QUADOBJ entries %s differ from triu(Q) %s.', ...
        name, mat2str(full(written)), mat2str(full(triu(Q))));
end
fprintf('[%s] QUADOBJ holds triu(Q) = %s\n', name, mat2str(full(triu(Q))));
end

function p = localCanonical(A, c, Q, b_l, b_u, l, u, sense, gamma)
% One problem form for comparisons: full matrices, Q symmetric with objective 1/2 x'Qx.
p = struct('A', full(A), 'c', full(c(:)), 'Q', full(Q), 'b_l', b_l(:), 'b_u', b_u(:), ...
    'l', l(:), 'u', u(:), 'sense', sense, 'gamma', gamma);
end

function p = localCanonicalGurobi(model)
% Gurobi's reading, with Gurobi's range columns (named MPS_Rg<row>) folded back into the row.
n = numel(model.obj);
A = model.A;
b_l = model.rhs;
b_u = model.rhs;
b_l(model.sense == '<') = -inf;
b_u(model.sense == '>') = inf;
keep = true(n, 1);
for j = 1:n
    if startsWith(model.varnames{j}, 'MPS_Rg')
        [i, ~, v] = find(A(:, j));
        if numel(i) ~= 1 || v ~= 1 || model.sense(i) ~= '=' || model.obj(j) ~= 0
            error('make_toy_instances:range', 'Unexpected Gurobi range column %s.', ...
                model.varnames{j});
        end
        % row + r = rhs, lb <= r <= ub  <=>  rhs - ub <= row <= rhs - lb
        b_l(i) = model.rhs(i) - model.ub(j);
        b_u(i) = model.rhs(i) - model.lb(j);
        keep(j) = false;
    end
end
Q = sparse(n, n);
if isfield(model, 'Q')
    Q = model.Q + model.Q';
end
sense = 'min';
if isfield(model, 'modelsense') && strcmpi(model.modelsense, 'max')
    sense = 'max';
end
gamma = 0;
if isfield(model, 'objcon')
    gamma = model.objcon;
end
p = localCanonical(A(:, keep), model.obj(keep), Q(keep, keep), b_l, b_u, ...
    model.lb(keep), model.ub(keep), sense, gamma);
end

function p = localCanonicalMosek(prob)
n = numel(prob.c);
Q = sparse(n, n);
if isfield(prob, 'qosubi') && ~isempty(prob.qosubi)
    L = sparse(prob.qosubi, prob.qosubj, prob.qoval, n, n);
    Q = L + tril(L, -1)';
end
sense = 'min';
if startsWith(prob.objsense, 'max')
    sense = 'max';
end
p = localCanonical(prob.a, prob.c, Q, prob.blc, prob.buc, prob.blx, prob.bux, sense, ...
    prob.cfix);
end

function localRequireSame(name, what, p, q)
fields = fieldnames(p);
differing = fields(~cellfun(@(f) isequal(p.(f), q.(f)), fields));
if ~isempty(differing)
    error('make_toy_instances:readBack', '[%s] %s differs in: %s', name, what, ...
        strjoin(differing, ', '));
end
end

function prob = localMosekRead(name, file)
[rcode, res] = mosekopt(sprintf('read(%s) echo(0)', file));
localRequireOk(name, 'read', rcode);
prob = res.prob;
end

function check = localSolveMosekFile(file)
prob = localMosekRead(file, file);
check = localSolveMosek(prob, 'file');
end

function check = localSolveMosek(prob, how)
[~, version] = mosekopt('version echo(0)');
[rcode, res] = mosekopt(sprintf('%s echo(0)', prob.objsense), prob);
localRequireOk('solve', 'optimize', rcode);
sol = res.sol.itr;
if isfield(res.sol, 'bas')
    sol = res.sol.bas;
end
switch sol.prosta
    case 'PRIMAL_INFEASIBLE'
        status = 'Infeasible';
    case 'DUAL_INFEASIBLE'
        status = 'Unbounded';
    otherwise
        if strcmp(sol.solsta, 'OPTIMAL')
            status = 'Optimal';
        else
            status = sprintf('MOSEK %s / %s', sol.prosta, sol.solsta);
        end
end
objective = NaN;
if strcmp(status, 'Optimal')
    objective = sol.pobjval;
end
check = struct('tool', 'MOSEK', 'version', sprintf('%d.%d.%d', ...
    version.version.major, version.version.minor, version.version.revision), ...
    'how', how, 'status', status, 'objective', objective, 'x', sol.xx);
end

function check = localSolveGurobiFile(file)
check = localSolveGurobi(gurobi_read(file), 'file', inf);
end

function check = localSolveGurobi(model, how, n)
params = struct('OutputFlag', 0, 'DualReductions', 0, 'BarQCPConvTol', 1e-9);
result = gurobi(model, params);
switch result.status
    case 'OPTIMAL'
        status = 'Optimal';
    case 'INFEASIBLE'
        status = 'Infeasible';
    case 'UNBOUNDED'
        status = 'Unbounded';
    otherwise
        status = ['Gurobi ' result.status];
end
objective = NaN;
x = [];
if strcmp(status, 'Optimal')
    objective = result.objval;
    x = result.x(1:min(n, numel(result.x)));
end
versions = localToolVersions();
check = struct('tool', 'Gurobi', 'version', versions.gurobi, 'how', how, ...
    'status', status, 'objective', objective, 'x', x);
end

% =========================================================================
% JTASK: write, read back, solve
% =========================================================================
function prob = localMosekProb(d, sym)
prob = struct('c', d.c, 'a', d.A, 'blc', d.b_l, 'buc', d.b_u, 'blx', d.l, 'bux', d.u, ...
    'f', d.F, 'g', d.g, 'accs', localAccs(d.cones, sym));
end

function accs = localAccs(cones, sym)
% MOSEK accs: [domain, dimension] per cone; power cones add [number of alphas, alphas].
accs = [];
for i = 1:numel(cones)
    k = cones{i};
    switch k.kind
        case 'quadratic'
            code = sym.MSK_DOMAIN_QUADRATIC_CONE;
        case 'rotated quadratic'
            code = sym.MSK_DOMAIN_RQUADRATIC_CONE;
        case 'primal exponential'
            code = sym.MSK_DOMAIN_PRIMAL_EXP_CONE;
        case 'dual exponential'
            code = sym.MSK_DOMAIN_DUAL_EXP_CONE;
        case 'primal power'
            code = sym.MSK_DOMAIN_PRIMAL_POWER_CONE;
        case 'dual power'
            code = sym.MSK_DOMAIN_DUAL_POWER_CONE;
        case 'nonnegative'
            code = sym.MSK_DOMAIN_RPLUS;
        case 'zero'
            code = sym.MSK_DOMAIN_RZERO;
        case 'positive semidefinite'
            code = sym.MSK_DOMAIN_SVEC_PSD_CONE;
        otherwise
            error('make_toy_instances:cone', 'Unknown cone kind %s.', k.kind);
    end
    entry = [code, k.dimension];
    if contains(k.kind, 'power')
        entry = [entry, numel(k.alpha), k.alpha]; %#ok<AGROW>
    end
    accs = [accs, entry]; %#ok<AGROW>
end
end

function checks = localCheckJtaskFile(d, file, prob)
back = localMosekRead(d.name, file);
fields = {'c', 'a', 'blc', 'buc', 'blx', 'bux', 'f', 'g', 'accs'};
differing = fields(~cellfun(@(f) isfield(back, f) && localBits(prob.(f), back.(f)), fields));
if ~startsWith(back.objsense, localMosekSense(d.sense))
    differing{end + 1} = 'objective sense';
end
if ~isempty(differing)
    error('make_toy_instances:readBack', '[%s] JTASK read back differs in: %s', ...
        d.name, strjoin(differing, ', '));
end
fprintf('[%s] JTASK read back bit-for-bit equal (accs = %s)\n', d.name, mat2str(back.accs));
checks = {localSolveMosek(back, 'file')};
kinds = cellfun(@(k) k.kind, d.cones, 'UniformOutput', false);
if all(ismember(kinds, {'quadratic', 'rotated quadratic', 'nonnegative', 'zero'}))
    checks{end + 1} = localSolveGurobi(localGurobiConic(d), 'built from definition', ...
        numel(d.c));
end
end

function model = localGurobiConic(d)
% Gurobi model of a conic definition: Nonnegative rows Fx >= -g, Zero rows Fx = -g,
% (rotated) quadratic cones through auxiliary columns u with Fx - u = -g and one
% quadratic constraint per cone (sum_{i>=2} u_i^2 - u_1^2 <= 0 with u_1 >= 0;
% sum_{i>=3} u_i^2 - 2 u_1 u_2 <= 0 with u_1, u_2 >= 0).
n = numel(d.c);
[A, rhs, sense] = localRowsToSense(d.A, d.b_l, d.b_u);
auxDims = cellfun(@(k) k.dimension * any(strcmp(k.kind, ...
    {'quadratic', 'rotated quadratic'})), d.cones);
nAux = sum(auxDims);
N = n + nAux;
A = [A, sparse(size(A, 1), nAux)];
lb = [d.l; -inf(nAux, 1)];
ub = [d.u; inf(nAux, 1)];
quadcon = struct('Qc', {}, 'q', {}, 'rhs', {}, 'sense', {});
row = 0;
aux = n;
for i = 1:numel(d.cones)
    k = d.cones{i};
    r = row + (1:k.dimension);
    row = row + k.dimension;
    block = [d.F(r, :), sparse(k.dimension, nAux)];
    switch k.kind
        case 'nonnegative'
            blockSense = repmat('>', k.dimension, 1);
        case 'zero'
            blockSense = repmat('=', k.dimension, 1);
        otherwise
            u = aux + (1:k.dimension);
            aux = aux + k.dimension;
            block(:, u) = -speye(k.dimension);
            blockSense = repmat('=', k.dimension, 1);
            if strcmp(k.kind, 'quadratic')
                lb(u(1)) = 0;
                Qc = sparse(u, u, [-1, ones(1, k.dimension - 1)], N, N);
            else
                lb(u(1:2)) = 0;
                Qc = sparse([u(1), u(3:end)], [u(2), u(3:end)], ...
                    [-2, ones(1, k.dimension - 2)], N, N);
            end
            quadcon(end + 1) = struct('Qc', Qc, 'q', zeros(N, 1), 'rhs', 0, ...
                'sense', '<'); %#ok<AGROW>
    end
    A = [A; block]; %#ok<AGROW>
    rhs = [rhs; -d.g(r)]; %#ok<AGROW>
    sense = [sense; blockSense]; %#ok<AGROW>
end
model = struct('A', sparse(A), 'obj', [d.c; zeros(nAux, 1)], 'rhs', rhs, 'sense', sense, ...
    'lb', lb, 'ub', ub, 'modelsense', d.sense);
if ~isempty(quadcon)
    model.quadcon = quadcon;
end
end

function s = localMosekSense(sense)
if strcmp(sense, 'max')
    s = 'maximize';
else
    s = 'minimize';
end
end

% =========================================================================
% Agreement, records
% =========================================================================
function localRequireAgreement(name, check, expectedStatus, expectedObjective)
if ~strcmp(check.status, expectedStatus)
    error('make_toy_instances:status', '[%s] %s (%s) status %s, expected %s.', ...
        name, check.tool, check.how, check.status, expectedStatus);
end
if strcmp(expectedStatus, 'Optimal')
    delta = abs(check.objective - expectedObjective);
    if ~(delta <= 1e-6 * (1 + abs(expectedObjective)))
        error('make_toy_instances:objective', ...
            '[%s] %s (%s) objective %.17g, expected %.17g (|difference| %.3g).', ...
            name, check.tool, check.how, check.objective, expectedObjective, delta);
    end
end
fprintf('[%s] %s %s (%s): %s, objective %.17g\n', name, check.tool, check.version, ...
    check.how, check.status, check.objective);
end

function localRequireX(name, check, expectedX)
% Where the optimal x is unique, the solver's x must be near it (|x - x*| <= 1e-4 (1 + |x*|)).
if isscalar(expectedX) && isnan(expectedX)
    return
end
deviation = max(abs(check.x(:) - expectedX(:)) ./ (1 + abs(expectedX(:))));
if numel(check.x) ~= numel(expectedX) || ~(deviation <= 1e-4)
    error('make_toy_instances:x', '[%s] %s x = %s, expected %s.', name, check.tool, ...
        mat2str(check.x', 8), mat2str(expectedX', 8));
end
fprintf('[%s] %s x within %.2g (relative) of the unique optimum\n', name, check.tool, deviation);
end

function row = localRow(name, check, expectedStatus, expectedObjective)
row = struct('instance', string(name), 'tool', string(check.tool), ...
    'how', string(check.how), 'status', string(check.status), ...
    'objective', check.objective, 'expected_status', string(expectedStatus), ...
    'expected_objective', expectedObjective);
end

function rec = localRecord(d, file, checks, scriptRelPath, commit, clean, licenceText)
rec = struct();
rec.instance = d.name;
rec.class = d.class;
rec.format = d.format;
rec.file_sha256 = localSha256(file);
rec.construction = d.construction;
rec.sense = d.sense;
rec.m = size(d.A, 1);
rec.n = numel(d.c);
rec.nnz = nnz(d.A);
rec.k = size(d.F, 1);
rec.nnz_F = nnz(d.F);
rec.cones = d.cones;
rec.expected_status = d.expected_status;
rec.expected_objective = d.expected_objective;
rec.expected_x = d.expected_x;
rec.derivation = d.derivation;
rec.independent_checks = checks;
rec.extracting_script = scriptRelPath;
rec.optsolver_commit = commit;
rec.optsolver_worktree_clean = clean;
rec.licence = licenceText;
end

function text = localAppendChecks(text, checks)
% Add (or replace) "independent_checks" as the last field, leaving the rest untouched.
existing = strfind(text, sprintf(',\n  "independent_checks"'));
if ~isempty(existing)
    text = text(1:existing - 1);
else
    text = strtrim(text);
    if text(end) ~= '}'
        error('make_toy_instances:record', 'Provenance record does not end with "}".');
    end
    text = strtrim(text(1:end - 1));
end
block = jsonencode(checks, 'PrettyPrint', true);
block = strrep(block, newline, [newline '  ']);
text = sprintf('%s,\n  "independent_checks": %s\n}\n', text, block);
end

% =========================================================================
% Utilities
% =========================================================================
function tf = localBits(x, y)
% True when two numeric arrays have the same size, pattern and IEEE 754 bit patterns.
if issparse(x) || issparse(y)
    tf = isequal(size(x), size(y)) && nnz(x) == nnz(y);
    if tf
        [ix, jx, vx] = find(x);
        [iy, jy, vy] = find(y);
        tf = isequal(ix, iy) && isequal(jx, jy) && localBits(vx, vy);
    end
    return
end
x = full(double(x(:)));
y = full(double(y(:)));
tf = isequal(size(x), size(y)) && isequal(typecast(x, 'uint64'), typecast(y, 'uint64'));
end

function versions = localToolVersions()
% MOSEK from mosekopt('version'); Gurobi from the banner of a one-variable solve.
persistent cached
if isempty(cached)
    [~, v] = mosekopt('version echo(0)');
    cached.mosek = sprintf('%d.%d.%d', v.version.major, v.version.minor, v.version.revision);
    model = struct('A', sparse(1, 1), 'obj', 0, 'rhs', 0, 'sense', '=', 'lb', 0, 'ub', 1); %#ok<NASGU> used inside evalc
    banner = evalc('gurobi(model, struct(''OutputFlag'', 1))');
    token = regexp(banner, 'Gurobi Optimizer version (\S+)', 'tokens', 'once');
    if isempty(token)
        error('make_toy_instances:gurobiVersion', 'No Gurobi version in the solver banner.');
    end
    cached.gurobi = token{1};
end
versions = cached;
end

function localRequireOk(name, what, rcode)
if rcode ~= 0
    error('make_toy_instances:mosek', '[%s] mosekopt %s returned %d.', name, what, rcode);
end
end

function localWriteText(file, text)
fid = fopen(file, 'w');
if fid < 0
    error('make_toy_instances:write', 'Cannot open %s for writing.', file);
end
fprintf(fid, '%s', text);
fclose(fid);
end

function root = localRepoRoot()
% Upward search from this file for the folder holding .specify.
root = fileparts(mfilename('fullpath'));
while ~isfolder(fullfile(root, '.specify'))
    parent = fileparts(root);
    if strcmp(parent, root)
        error('make_toy_instances:root', 'No .specify folder above %s.', mfilename('fullpath'));
    end
    root = parent;
end
end

function h = localSha256(file)
% SHA-256 of a file as lower-case hexadecimal, through the Java runtime.
md = java.security.MessageDigest.getInstance('SHA-256');
fid = fopen(file, 'r');
while true
    chunk = fread(fid, 2^24, '*uint8');
    if isempty(chunk)
        break
    end
    md.update(typecast(chunk, 'int8'));
end
fclose(fid);
d = typecast(md.digest(), 'uint8');
h = lower(reshape(dec2hex(d, 2)', 1, []));
end

function c = localGit(repo)
% HEAD commit of a git repository.
[status, out] = system(sprintf('git -C "%s" rev-parse HEAD', repo));
if status ~= 0
    error('make_toy_instances:git', 'git rev-parse failed in %s: %s', repo, out);
end
c = strtrim(out);
end
