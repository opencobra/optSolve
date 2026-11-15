function extract_idopaneuroc_fixtures()
% extract_idopaneuroc_fixtures  Freeze the iDopaNeuroC instances as MPS test fixtures.
%
% Usage
%   setenv('OPTSOLVER_VK_REPO', '<path to the variational kinetics repository>')
%   extract_idopaneuroc_fixtures()
%
% For every LP fixture named in this repository's project declaration
% (.specify/memory/project.yml, key test_corpus.lp_instances), this function
%   1. looks up the fixture's variational kinetics source instance in
%      test_corpus.lp_source_instances (fixture name -> source instance name);
%   2. locates the frozen corpus with the variational kinetics locator
%      vkProblemInstanceDir (reused, not copied);
%   3. loads the source instance's COBRA structure `model`;
%   4. builds a Gurobi MATLAB model structure field by field:
%        A = S, rhs = b, sense from csense (E -> '=', L -> '<', G -> '>'),
%        lb, ub, obj = c, modelsense from osenseStr;
%   5. writes tests/fixtures/<fixture>.mps with gurobi_write;
%   6. reads the file back with gurobi_read and requires A, rhs, sense, lb, ub
%      and obj to be bit-for-bit identical to what was written (error otherwise);
%   7. writes tests/fixtures/<fixture>.provenance.json (fields of the "Fixture
%      provenance record" in specs/001-core-gurobi-lp/data-model.md, plus
%      source_instance and renamed_from naming the source instance).
%
% Inputs (environment, resolved at run time and never stored)
%   OPTSOLVER_VK_REPO   absolute path to the variational kinetics repository.
%
% The Gurobi MATLAB interface is expected at /usr/local/bin/gurobi1303/linux64/matlab
% unless gurobi_write is already on the MATLAB path.
%
% Errors carry an identifier; nothing is written for an instance whose round trip
% fails, and the function stops at the first failure.

repoRoot = fileparts(fileparts(fileparts(mfilename('fullpath'))));
scriptRelPath = 'tools/corpus/extract_idopaneuroc_fixtures.m';

vkRepo = getenv('OPTSOLVER_VK_REPO');
if isempty(vkRepo) || ~isfolder(vkRepo)
    error('extract_idopaneuroc_fixtures:noVkRepo', ...
        'Set OPTSOLVER_VK_REPO to the variational kinetics repository (got "%s").', vkRepo);
end

% Reuse the variational kinetics corpus locator and its path resolver.
addpath(fullfile(vkRepo, 'src', 'matlab'));
addpath(fullfile(vkRepo, 'src', 'matlab', 'experiment', 'corpus'));
if isempty(which('gurobi_write'))
    addpath('/usr/local/bin/gurobi1303/linux64/matlab');
end
if isempty(which('gurobi_write'))
    error('extract_idopaneuroc_fixtures:noGurobi', 'gurobi_write is not on the MATLAB path.');
end

% Instance names come from the single project declaration, never restated here.
[status, out] = system(sprintf('python3 -I "%s" --json', ...
    fullfile(repoRoot, '.specify', 'scripts', 'python', 'project_params.py')));
if status ~= 0
    error('extract_idopaneuroc_fixtures:params', 'project_params.py failed: %s', out);
end
params = jsondecode(out);
instances = cellstr(params.test_corpus.lp_instances);
sourceMap = params.test_corpus.lp_source_instances;   % struct: fixture name -> source name
fixtureDir = fullfile(repoRoot, params.test_corpus.fixtures);

corpusDir = vkProblemInstanceDir();
if isempty(corpusDir)
    error('extract_idopaneuroc_fixtures:noCorpus', 'vkProblemInstanceDir found no corpus.');
end

vkCommit = localGit(vkRepo);
optSolverCommit = localGit(repoRoot);
[~, dirty] = system(sprintf('git -C "%s" status --porcelain', repoRoot));
optSolverClean = isempty(strtrim(dirty));
seeds = [1; 2; 3];

for k = 1:numel(instances)
    name = instances{k};
    if ~isfield(sourceMap, name)
        error('extract_idopaneuroc_fixtures:noSource', ...
            'test_corpus.lp_source_instances has no entry for fixture "%s".', name);
    end
    sourceName = sourceMap.(name);
    sourceFile = fullfile(corpusDir, [sourceName '.mat']);
    fprintf('[%s] loading source instance %s from %s\n', name, sourceName, sourceFile);
    s = load(sourceFile, 'model');
    model = s.model;

    g = localGurobiModel(model);
    [m, n] = size(g.A);

    mpsFile = fullfile(fixtureDir, [name '.mps']);
    gurobi_write(g, mpsFile);
    back = gurobi_read(mpsFile);
    localRequireIdentical(name, g, back);
    fprintf('[%s] m = %d, n = %d, nnz = %d, round trip bit-for-bit identical\n', ...
        name, m, n, nnz(g.A));

    rec = struct();
    rec.instance = name;
    rec.source_repository = 'variationalKinetics';
    rec.source_commit = vkCommit;
    rec.source_instance = sourceName;
    rec.renamed_from = sourceName;
    rec.source_file = [sourceName '.mat'];
    rec.source_sha256 = localSha256(sourceFile);
    rec.extracting_script = scriptRelPath;
    rec.optsolver_commit = optSolverCommit;
    rec.optsolver_worktree_clean = optSolverClean;
    rec.m = m;
    rec.n = n;
    rec.nnz = nnz(g.A);
    rec.objective_sense = model.osenseStr;
    rec.seeds = seeds;
    rec.citation = model.PleaseCite;
    rec.licence = 'not verified (maintainer decision 2026-11-14)';
    rec.mps_sha256 = localSha256(mpsFile);
    rec.array_sha256 = localArrayHashes(g);
    rec.round_trip = 'gurobi_read of the written file is bit-for-bit identical in A, rhs, sense, lb, ub, obj';

    fid = fopen(fullfile(fixtureDir, [name '.provenance.json']), 'w');
    fprintf(fid, '%s\n', jsonencode(rec, 'PrettyPrint', true));
    fclose(fid);
    fprintf('[%s] wrote %s and provenance record\n', name, mpsFile);
end
end

% =========================================================================
function g = localGurobiModel(model)
% Builds a Gurobi MATLAB model structure from a COBRA model, field by field.
%
g.A = sparse(model.S);
g.rhs = full(double(model.b(:)));
csense = model.csense(:);
sense = repmat('=', numel(csense), 1);
sense(csense == 'L') = '<';
sense(csense == 'G') = '>';
if any(~ismember(csense, 'ELG'))
    error('extract_idopaneuroc_fixtures:csense', 'csense holds a value other than E, L, G.');
end
g.sense = sense;
g.lb = full(double(model.lb(:)));
g.ub = full(double(model.ub(:)));
g.obj = full(double(model.c(:)));
switch model.osenseStr
    case 'max'
        g.modelsense = 'max';
    case 'min'
        g.modelsense = 'min';
    otherwise
        error('extract_idopaneuroc_fixtures:osense', 'Unknown osenseStr "%s".', model.osenseStr);
end
end

% =========================================================================
function localRequireIdentical(name, g, back)
% Requires every array read back from the MPS file to equal what was written, bit for bit.
[iw, jw, vw] = find(g.A);
[ir, jr, vr] = find(back.A);
checks = { ...
    'A size', isequal(size(g.A), size(back.A)); ...
    'A pattern', isequal(iw, ir) && isequal(jw, jr); ...
    'A values', isequal(typecast(vw, 'uint64'), typecast(vr, 'uint64')); ...
    'rhs', localBits(g.rhs, back.rhs); ...
    'sense', isequal(g.sense(:), back.sense(:)); ...
    'lb', localBits(g.lb, back.lb); ...
    'ub', localBits(g.ub, back.ub); ...
    'obj', localBits(g.obj, back.obj); ...
    'modelsense', strcmpi(g.modelsense, back.modelsense)};
failed = checks(~[checks{:, 2}], 1);
if ~isempty(failed)
    error('extract_idopaneuroc_fixtures:roundTrip', ...
        '[%s] gurobi_read differs from what was written in: %s', name, strjoin(failed, ', '));
end
end

function tf = localBits(a, b)
% True when two double vectors have identical IEEE 754 bit patterns.
a = full(double(a(:)));
b = full(double(b(:)));
tf = isequal(size(a), size(b)) && isequal(typecast(a, 'uint64'), typecast(b, 'uint64'));
end

% =========================================================================
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

function h = localSha256Bytes(bytes)
% SHA-256 of a uint8 vector as lower-case hexadecimal.
md = java.security.MessageDigest.getInstance('SHA-256');
if ~isempty(bytes)
    md.update(typecast(bytes(:), 'int8'));
end
d = typecast(md.digest(), 'uint8');
h = lower(reshape(dec2hex(d, 2)', 1, []));
end

function hashes = localArrayHashes(g)
% SHA-256 of each array's little-endian bytes, in the layout the Rust MPS reader produces:
% A in compressed sparse column form with 0-based row indices sorted within each column
% (A_col_ptrs and A_row_indices as unsigned 64-bit integers, A_values as doubles), the row
% senses as ASCII bytes ('=', '<', '>'), and b, l, u, c as doubles. A test parses the MPS
% file and compares these, so the fixture is shown to hold the source model's arrays bit for bit.
[~, ~, endian] = computer();
if endian ~= 'L'
    error('extract_idopaneuroc_fixtures:endian', 'Array hashes assume a little-endian machine.');
end
[i, j, v] = find(g.A);              % column-major order: sorted by column, then row
n = size(g.A, 2);
colPtrs = [0; cumsum(accumarray(j, 1, [n 1]))];
le = @(x) typecast(x(:), 'uint8');
hashes = struct();
hashes.A_col_ptrs = localSha256Bytes(le(uint64(colPtrs)));
hashes.A_row_indices = localSha256Bytes(le(uint64(i - 1)));
hashes.A_values = localSha256Bytes(le(double(v)));
hashes.sense = localSha256Bytes(uint8(g.sense(:)));
hashes.b = localSha256Bytes(le(g.rhs));
hashes.l = localSha256Bytes(le(g.lb));
hashes.u = localSha256Bytes(le(g.ub));
hashes.c = localSha256Bytes(le(g.obj));
end

function c = localGit(repo)
% HEAD commit of a git repository.
[status, out] = system(sprintf('git -C "%s" rev-parse HEAD', repo));
if status ~= 0
    error('extract_idopaneuroc_fixtures:git', 'git rev-parse failed in %s: %s', repo, out);
end
c = strtrim(out);
end
