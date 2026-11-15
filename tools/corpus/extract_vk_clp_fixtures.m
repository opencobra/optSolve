function summary = extract_vk_clp_fixtures(options)
% extract_vk_clp_fixtures  Freeze variational kinetics conic problems as CLP test fixtures.
%
% Usage
%   setenv('OPTSOLVER_VK_REPO', '<path to the variational kinetics repository>')
%   summary = extract_vk_clp_fixtures()
%   summary = extract_vk_clp_fixtures(CheckOnly = true, ScratchDir = '<folder>')
%
% The CLP test problem is the problem optimizeVKmodel passes to solveCobraCLP on the
% single route (specs/002-mosek-lp-qp-clp/research.md, R11). For every instance named in
% this repository's project declaration (.specify/memory/project.yml, keys
% test_corpus.clp_instances and test_corpus.clp_heavy_instances), this function
%   1. loads the frozen instance (model, param, problemInstance) from the variational
%      kinetics corpus located by vkProblemInstanceDir (reused, not copied);
%   2. sets param.solveStrategy = 'single', printLevel = 0, saveProb = 1, diary = 0 and
%      plotFigures = 0, restores the stored random number generator state, and runs
%      optimizeVKmodel(model, param) with the current folder set to a scratch folder,
%      so the problem solveCobraCLP saves before calling mosekopt lands there;
%   3. takes sol.prob (fields A, bl, bu, c, xl, xu, F, g, accs) and checks that the cone
%      dimensions in accs sum to the number of rows of F, that every domain code is a
%      primal exponential or rotated quadratic cone, and that sol.prob equals the saved
%      MOSEK problem bit for bit (after MOSEK's field renaming A -> a, F -> f, bl -> blc,
%      bu -> buc, xl -> blx, xu -> bux);
%   4. writes the problem with MOSEK's own writer to the scratch folder in the format
%      declared by test_corpus.clp_format (the file extension selects MOSEK's writer),
%      reads it back with mosekopt('read(...)') and requires a, c, blc, buc, blx, bux, f,
%      g, accs and the objective sense to be bit-for-bit identical; it stops otherwise;
%   5. re-solves the extracted problem once with the saved MOSEK command and parameters
%      to record the optimizer time;
%   6. unless CheckOnly is true, copies the file to tests/fixtures/clp/<instance>.<format>
%      and writes tests/fixtures/clp/<instance>.provenance.json.
%
% Name-value options
%   CheckOnly   (false) run every step, report round-trip failures without stopping,
%               and write nothing under tests/fixtures.
%   ScratchDir  (tempname) folder for MOSEK's saved problem and the trial file.
%   Instances   (all declared) subset of the declared instance names.
%   Format      (test_corpus.clp_format) file extension handed to MOSEK's writer, for
%               probing another format with CheckOnly; fixtures use the declared one.
%
% Inputs (environment, resolved at run time and never stored)
%   OPTSOLVER_VK_REPO   absolute path to the variational kinetics repository, already
%                       set up on the MATLAB path (initVK) with COBRA and MOSEK.
%
% Output
%   summary   table, one row per instance: dimensions, cone counts, expected status,
%             objective, round-trip result.
%
% Errors carry an identifier. Nothing is written under tests/fixtures for an instance
% whose checks fail.

arguments
    options.CheckOnly (1, 1) logical = false
    options.ScratchDir {mustBeTextScalar} = tempname
    options.Instances {mustBeText} = {}
    options.Format {mustBeTextScalar} = ''
end

repoRoot = fileparts(fileparts(fileparts(mfilename('fullpath'))));
scriptRelPath = 'tools/corpus/extract_vk_clp_fixtures.m';
roundTripFields = {'a', 'c', 'blc', 'buc', 'blx', 'bux', 'f', 'g', 'accs'};

vkRepo = getenv('OPTSOLVER_VK_REPO');
if isempty(vkRepo) || ~isfolder(vkRepo)
    error('extract_vk_clp_fixtures:noVkRepo', ...
        'Set OPTSOLVER_VK_REPO to the variational kinetics repository (got "%s").', vkRepo);
end
if isempty(which('optimizeVKmodel')) || isempty(which('mosekopt'))
    error('extract_vk_clp_fixtures:path', ...
        'optimizeVKmodel and mosekopt must be on the MATLAB path (run initVK first).');
end
addpath(fullfile(vkRepo, 'src', 'matlab', 'experiment', 'corpus'));

% Instance names and format come from the single project declaration.
[status, out] = system(sprintf('python3 -I "%s" --json', ...
    fullfile(repoRoot, '.specify', 'scripts', 'python', 'project_params.py')));
if status ~= 0
    error('extract_vk_clp_fixtures:params', 'project_params.py failed: %s', out);
end
params = jsondecode(out);
corpus = params.test_corpus;
declared = [cellstr(corpus.clp_instances); cellstr(corpus.clp_heavy_instances)];
heavy = cellstr(corpus.clp_heavy_instances);
instances = declared;
if ~isempty(options.Instances)
    instances = cellstr(options.Instances);
    unknown = setdiff(instances, declared);
    if ~isempty(unknown)
        error('extract_vk_clp_fixtures:undeclared', ...
            'Not declared in project.yml: %s', strjoin(unknown, ', '));
    end
end
format = char(corpus.clp_format);
if strlength(options.Format) > 0
    if ~options.CheckOnly && ~strcmp(char(options.Format), format)
        error('extract_vk_clp_fixtures:format', ...
            'Format may differ from test_corpus.clp_format (%s) only with CheckOnly.', format);
    end
    format = char(options.Format);
end
fixtureDir = fullfile(repoRoot, corpus.fixtures, 'clp');

corpusDir = vkProblemInstanceDir();
if isempty(corpusDir)
    error('extract_vk_clp_fixtures:noCorpus', 'vkProblemInstanceDir found no corpus.');
end
scratchRoot = char(options.ScratchDir);
if ~isfolder(scratchRoot)
    mkdir(scratchRoot);
end

vkCommit = localGit(vkRepo);
optSolverCommit = localGit(repoRoot);
[~, dirty] = system(sprintf('git -C "%s" status --porcelain', repoRoot));
optSolverClean = isempty(strtrim(dirty));
[~, mosekVersion] = mosekopt('version');
mosekVersionText = sprintf('%d.%d.%d', mosekVersion.version.major, ...
    mosekVersion.version.minor, mosekVersion.version.revision);
[~, symbols] = mosekopt('symbcon echo(0)');
expectedDomains = [symbols.symbcon.MSK_DOMAIN_PRIMAL_EXP_CONE, ...
    symbols.symbcon.MSK_DOMAIN_RQUADRATIC_CONE];
domainNames = containers.Map( ...
    {symbols.symbcon.MSK_DOMAIN_PRIMAL_EXP_CONE, symbols.symbcon.MSK_DOMAIN_RQUADRATIC_CONE}, ...
    {'primal exponential', 'rotated quadratic'});

rows = cell(numel(instances), 1);
for k = 1:numel(instances)
    name = instances{k};
    sourceFile = fullfile(corpusDir, [name '.mat']);
    fprintf('[%s] loading %s\n', name, sourceFile);
    stored = load(sourceFile, 'model', 'param', 'problemInstance');
    workDir = fullfile(scratchRoot, name);
    if isfolder(workDir)
        rmdir(workDir, 's');
    end
    mkdir(workDir);

    % Step 2: the variational kinetics single route, run inside the scratch folder.
    param = stored.param;
    param.solveStrategy = 'single';
    param.printLevel = 0;
    param.saveProb = 1;
    param.diary = 0;
    param.plotFigures = 0;
    rng(stored.problemInstance.rngState);
    previousDir = cd(workDir);
    restoreDir = onCleanup(@() cd(previousDir));
    [~, sol] = evalc('optimizeVKmodel(stored.model, param)');
    clear restoreDir
    prob = sol.prob;

    % Step 3: structural checks and bit equality with the problem MOSEK received.
    saved = dir(fullfile(workDir, '*_probBeforeMosekopt.mat'));
    if numel(saved) ~= 1
        error('extract_vk_clp_fixtures:savedProb', ...
            '[%s] expected one saved MOSEK problem in %s, found %d.', name, workDir, numel(saved));
    end
    clp = load(fullfile(workDir, saved(1).name), 'CLP');
    clp = clp.CLP;
    mosekProb = localMosekProb(prob);
    localRequireIdentical(name, 'sol.prob against the saved MOSEK problem', ...
        mosekProb, clp.prob, roundTripFields);
    coneTypes = double(prob.accs(1:2:end));
    coneDims = double(prob.accs(2:2:end));
    if sum(coneDims) ~= size(prob.F, 1)
        error('extract_vk_clp_fixtures:coneRows', ...
            '[%s] cone dimensions sum to %d, F has %d rows.', name, sum(coneDims), size(prob.F, 1));
    end
    if any(~ismember(coneTypes, expectedDomains))
        error('extract_vk_clp_fixtures:domain', '[%s] unexpected domain codes: %s', ...
            name, mat2str(unique(coneTypes(~ismember(coneTypes, expectedDomains)))));
    end
    objectiveSense = localSense(clp.cmd);

    % Step 4: MOSEK writes the problem; reading it back must give the same arrays.
    trialFile = fullfile(workDir, [name '.' format]);
    [rcode, ~] = mosekopt(sprintf('%s write(%s) echo(0)', objectiveSense, trialFile), ...
        clp.prob, clp.param);
    localRequireOk(name, 'write', rcode);
    [rcode, back] = mosekopt(sprintf('read(%s) echo(0)', trialFile));
    localRequireOk(name, 'read', rcode);
    differing = localDiffering(clp.prob, back.prob, roundTripFields);
    if ~strcmpi(back.prob.objsense(1:3), objectiveSense(1:3))
        differing{end + 1} = 'objective sense'; %#ok<AGROW>
    end
    roundTripOk = isempty(differing);
    if roundTripOk
        fprintf('[%s] %s round trip bit-for-bit identical\n', name, format);
    else
        fprintf('[%s] %s round trip DIFFERS in: %s\n', name, format, strjoin(differing, ', '));
        if ~options.CheckOnly
            error('extract_vk_clp_fixtures:roundTrip', ...
                '[%s] reading the %s file back differs in: %s. Nothing written.', ...
                name, format, strjoin(differing, ', '));
        end
    end

    % Step 5: one re-solve of the extracted problem for the optimizer time (N = 1).
    [~, resolve] = mosekopt([clp.cmd ' info'], clp.prob, clp.param);
    optimizerTime = resolve.info.MSK_DINF_OPTIMIZER_TIME;

    cones = localConeSummary(coneTypes, coneDims, domainNames);
    row = struct('instance', name, 'm', size(prob.A, 1), 'n', size(prob.A, 2), ...
        'nnz_A', nnz(prob.A), 'k', size(prob.F, 1), 'nnz_F', nnz(prob.F), ...
        'n_cones', numel(coneTypes), ...
        'n_exp_cones', nnz(coneTypes == symbols.symbcon.MSK_DOMAIN_PRIMAL_EXP_CONE), ...
        'n_rquad_cones', nnz(coneTypes == symbols.symbcon.MSK_DOMAIN_RQUADRATIC_CONE), ...
        'rquad_dims', string(mat2str(unique(coneDims(coneTypes == ...
        symbols.symbcon.MSK_DOMAIN_RQUADRATIC_CONE)))), ...
        'file_bytes', localBytes(trialFile), 'stat', sol.stat, 'origStat', string(sol.origStat), ...
        'objective', sol.obj, 'vk_solve_time_s', sol.time, ...
        'optimizer_time_s', optimizerTime, 'round_trip_ok', roundTripOk, ...
        'round_trip_differs', string(strjoin(differing, ', ')));
    rows{k} = row;
    fprintf(['[%s] m = %d, n = %d, nnz(A) = %d, k = %d, nnz(F) = %d, cones = %d, ' ...
        'stat = %d, origStat = %s, objective = %.17g, VK solve %.3f s, optimizer %.3f s\n'], ...
        name, row.m, row.n, row.nnz_A, row.k, row.nnz_F, row.n_cones, row.stat, ...
        row.origStat, row.objective, row.vk_solve_time_s, row.optimizer_time_s);

    if options.CheckOnly || ~roundTripOk
        continue
    end

    % Step 6: the fixture and its provenance record.
    if ~isfolder(fixtureDir)
        mkdir(fixtureDir);
    end
    fixtureFile = fullfile(fixtureDir, [name '.' format]);
    copyfile(trialFile, fixtureFile);
    rec = struct();
    rec.instance = name;
    rec.problem_class = 'CLP';
    rec.tier = localTier(name, heavy);
    rec.statement = ['the problem optimizeVKmodel passes to solveCobraCLP ' ...
        '(solveStrategy = ''single'')'];
    rec.source_repository = 'variationalKinetics';
    rec.source_commit = vkCommit;
    rec.source_file = [name '.mat'];
    rec.source_sha256 = localSha256(sourceFile);
    rec.source_param_overrides = struct('solveStrategy', 'single', 'printLevel', 0, ...
        'saveProb', 1, 'diary', 0, 'plotFigures', 0);
    rec.extracting_script = scriptRelPath;
    rec.optsolver_commit = optSolverCommit;
    rec.optsolver_worktree_clean = optSolverClean;
    rec.m = row.m;
    rec.n = row.n;
    rec.nnz_A = row.nnz_A;
    rec.k = row.k;
    rec.nnz_F = row.nnz_F;
    rec.cones = cones;
    rec.objective_sense = objectiveSense;
    rec.mosek_version = mosekVersionText;
    rec.mosek_command = clp.cmd;
    rec.mosek_parameters = clp.param;
    rec.expected_stat = row.stat;
    rec.expected_origStat = char(row.origStat);
    rec.expected_objective = row.objective;
    rec.vk_solve_time_s = row.vk_solve_time_s;
    rec.optimizer_time_s = row.optimizer_time_s;
    rec.timing_replicates = 1;
    rec.citation = localCitation(stored.model);
    rec.licence = 'not verified (maintainer decision 2026-11-14, extended at Gate 2A 2026-11-15)';
    rec.format = format;
    rec.format_decision = ['format: JTASK, maintainer decision 2026-11-15 (CBF does not ' ...
        'round-trip the linear block and bounds)'];
    rec.([format '_sha256']) = localSha256(fixtureFile);
    rec.round_trip = sprintf(['mosekopt read of the written %s file is bit-for-bit ' ...
        'identical in %s and the objective sense'], format, strjoin(roundTripFields, ', '));
    fid = fopen(fullfile(fixtureDir, [name '.provenance.json']), 'w');
    fprintf(fid, '%s\n', jsonencode(rec, 'PrettyPrint', true));
    fclose(fid);
    fprintf('[%s] wrote %s and provenance record\n', name, fixtureFile);
end
summary = struct2table([rows{:}]);
end

% =========================================================================
function p = localMosekProb(prob)
% sol.prob under MOSEK's field names, as solveCobraCLP renames them before mosekopt.
p = struct('a', prob.A, 'blc', prob.bl(:), 'buc', prob.bu(:), 'c', prob.c(:), ...
    'blx', prob.xl(:), 'bux', prob.xu(:), 'accs', prob.accs, 'f', prob.F, 'g', prob.g(:));
end

function sense = localSense(cmd)
% Objective sense of a mosekopt command string.
if contains(cmd, 'maximize')
    sense = 'maximize';
else
    sense = 'minimize';
end
end

function localRequireIdentical(name, what, p, q, fields)
% Errors unless every listed field is bit-for-bit identical.
differing = localDiffering(p, q, fields);
if ~isempty(differing)
    error('extract_vk_clp_fixtures:notIdentical', '[%s] %s differs in: %s', ...
        name, what, strjoin(differing, ', '));
end
end

function differing = localDiffering(p, q, fields)
% Names of the fields whose sizes, sparsity patterns or IEEE 754 bit patterns differ.
differing = {};
for i = 1:numel(fields)
    f = fields{i};
    if ~isfield(p, f) || ~isfield(q, f) || ~localBits(p.(f), q.(f))
        differing{end + 1} = f; %#ok<AGROW>
    end
end
end

function tf = localBits(x, y)
% True when two numeric arrays have the same size, pattern and bit patterns.
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

function localRequireOk(name, what, rcode)
if rcode ~= 0
    error('extract_vk_clp_fixtures:mosek', '[%s] mosekopt %s returned %d.', name, what, rcode);
end
end

function cones = localConeSummary(types, dims, domainNames)
% One entry per (domain, dimension) pair, with its count.
pairs = unique([types(:), dims(:)], 'rows');
cones = struct('type', {}, 'mosek_domain', {}, 'dimension', {}, 'count', {});
for i = 1:size(pairs, 1)
    cones(i).type = domainNames(pairs(i, 1));
    cones(i).mosek_domain = pairs(i, 1);
    cones(i).dimension = pairs(i, 2);
    cones(i).count = nnz(types(:) == pairs(i, 1) & dims(:) == pairs(i, 2));
end
end

function b = localBytes(file)
info = dir(file);
b = info.bytes;
end

function tier = localTier(name, heavy)
if ismember(name, heavy)
    tier = 'heavy';
else
    tier = 'licensed';
end
end

function c = localCitation(model)
if isfield(model, 'PleaseCite')
    c = model.PleaseCite;
else
    c = 'not recorded in the source model';
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
    error('extract_vk_clp_fixtures:git', 'git rev-parse failed in %s: %s', repo, out);
end
c = strtrim(out);
end
