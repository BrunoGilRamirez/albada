import type { Resource } from "i18next";
import type { AppLocale } from "./locale";

// Statically bundled catalogs: no runtime catalog loading, no network
// requests, offline-safe. Vite embeds these JSON modules in the build graph.
import enUS from "./locales/en-US.json";
import esES from "./locales/es-ES.json";
import deDE from "./locales/de-DE.json";
import frFR from "./locales/fr-FR.json";

/**
 * Typed shape of the bundled translation catalog. Every locale catalog must
 * match this interface exactly at compile time (missing keys fail `tsc`), and the
 * runtime parity test additionally checks plural variants and interpolation
 * placeholders across locales.
 *
 * `common.taskCount_one/_other` are i18next plural forms: call sites use the
 * base key `common.taskCount` with a numeric `count` option, which the type
 * system and the runtime both resolve through CLDR plural rules. The same
 * `_one/_other` pattern applies to `shell.bell.unread`,
 * `today.ritual.capacityHintPreferred`, `common.progress.pointsSummary`, and
 * the 8.6.10 `journal.selectedCount`/`listedCount`/`archiveConfirm`/`archived`/
 * `markedExportable`/`markedPrivate` keys. `capacityHintPreferred` and
 * `pointsSummary` use numeric `count` for plural selection with a
 * locale-formatted `countDisplay` for the visible value; the journal keys
 * pluralize on and display the plain numeric `count`. The 8.6.11 keys
 * `common.exportPreview.itemCount`, `common.exportHistory.count`, and
 * `notifications.rule.sentToday` pluralize the same way on a plain `count`.
 * Keys whose count is always parenthesized (`common.exportJob.diagnostics`,
 * `common.exportJob.localArtifacts`, `import.report.showWarnings`,
 * `import.report.reviewItems`) interpolate `count` without plural variants.
 *
 * Section ownership (8.6.7 foundation + 8.6.8 shared shell/Today/Calendar +
 * 8.6.9 Goals/Projects/Tasks execution screens + 8.6.10 Journal/Therapy +
 * 8.6.11 Settings/Import-Export/notifications/shared feedback +
 * 8.7.13 read-only habit analytics):
 * `settings.language` and `errors` are foundation keys; `common` holds copy
 * reused across screens (generic actions, shared form-field labels, domain
 * enum labels used by more than one feature area, shared loading/saving
 * feedback, the shared Markdown preview chrome and the editor toolbar chrome
 * passed to `MarkdownField`/`MarkdownPreview`/`useMarkdownEditorChrome` by the
 * localized forms, the export-page chrome shared by the Journal and Therapy
 * export screens, the progress detail surface rendered by the Goals
 * and Projects screens, and — since 8.6.11 — the shared export preview/job/
 * history and job-progress chrome plus the error-boundary feedback);
 * `journal` and `therapy` hold screen-specific copy for
 * the 8.6.10 journaling and therapy routes, their forms, reference editors,
 * template-field chrome, and privacy/exportability labels — therapy copy
 * stays inside the `therapy` namespace and is never rendered on shared
 * surfaces; `shell` holds
 * the application frame (sidebar navigation, theme toggle, command palette,
 * notification bell controls, breadcrumb); `today` and `calendar` hold
 * screen-specific copy for those 8.6.8 routes and their panels; `analytics`
 * holds screen-specific copy for the 8.7.13 read-only habit analytics route
 * (range controls, the effective-range line, coverage and outcome labels, the
 * rate definition, the weekly trend table, and the routine review action) and
 * never carries journal or therapy copy; `goals`,
 * `projects`, and `tasks` hold screen-specific copy for the 8.6.9 execution
 * and planning routes, their forms, and their planning panels;
 * `settings` (besides `language`) holds the 8.6.11 Settings, availability,
 * commitment, backup-reminder, and legacy-import screens; `notifications`
 * holds the 8.6.11 notification preferences, rule form, history panel, and
 * first-startup consent dialog; `import` holds the 8.6.11 legacy-import
 * report view, warning rows, batch status labels, and the selection
 * fingerprint hint. Enum-keyed
 * maps use the raw stored value as the key (`in_progress`, `after_meal`) so
 * components can build keys as `t(\`common.labels.taskStatus.${task.status}\`)`.
 * Exception to "UI copy only": `today.ritual.capacityNotePrefill*` and
 * `today.ritual.planningBlockLabel` are application-generated content
 * defaults translated at creation time in the active locale, then stored
 * verbatim (interpolated clock times and `energy_expectation` stay verbatim).
 * User-authored values (titles, Markdown sources, notes) are never keyed or
 * transformed by the catalogs in any section. Values that are stored data
 * rather than copy — privacy scopes, notification target entity types, backup
 * kinds inside user-configured messages, and the fixed native delivery text
 * `A validated backup is due.` — likewise stay verbatim; only the chrome
 * around them is keyed.
 */
export interface Translation {
  settings: {
    language: {
      label: string;
      description: string;
      system: string;
      "en-US": string;
      "es-ES": string;
      "de-DE": string;
      "fr-FR": string;
      saved: string;
    };
    title: string;
    subtitle: string;
    sections: {
      aria: string;
      commitments: string;
      availability: string;
      import: string;
      notifications: string;
    };
    workspace: {
      label: string;
      value: string;
    };
    warningWindow: {
      label: string;
      help: string;
      invalid: string;
      saved: string;
    };
    archived: {
      title: string;
      help: string;
      empty: string;
      restored: string;
    };
    backups: {
      title: string;
      help: string;
      create: string;
      creating: string;
      creatingHelp: string;
      restoring: string;
      created: string;
      deleted: string;
      delete: string;
      deleteAria: string;
      deleteConfirm: string;
      restoreAria: string;
      kinds: {
        recovery: string;
        user: string;
      };
      restoreKind: {
        recovery: string;
        user: string;
      };
      restoreConfirmTitle: string;
      restoreConfirmDetail: string;
      restoreConfirmWarning: string;
      restoreConfirmPrivacy: string;
      restoreFailed: string;
      restoreMissingChecksum: string;
      restored: string;
      schemaUnknown: string;
    };
    backupReminder: {
      aria: string;
      title: string;
      help: string;
      enableAria: string;
      enabled: string;
      interval: string;
      intervalHelp: string;
      intervalWholeError: string;
      intervalRangeError: string;
      lastValidated: string;
      lastValidatedNone: string;
      timestampNote: string;
      safeMessageNote: string;
      scheduleNote: string;
      saved: string;
      saveSubmit: string;
    };
    diagnostics: {
      title: string;
      help: string;
      compilerVersion: string;
      capability: string;
      statusValid: string;
      statusInvalid: string;
      unavailableResources: string;
      template: string;
      fonts: string;
      fontsAvailable: string;
      environment: string;
      sqliteVersion: string;
      tauriVersion: string;
      osVersion: string;
      unavailable: string;
    };
    availability: {
      title: string;
      subtitle: string;
      weekday: string;
      energy: string;
      energyPlaceholder: string;
      start: string;
      end: string;
      preferredLength: string;
      addSubmit: string;
      updateSubmit: string;
      cancelEdit: string;
      profiles: string;
      profilesEmpty: string;
      added: string;
      updated: string;
      blockLength: string;
      noPreferredLength: string;
    };
    commitments: {
      title: string;
      subtitle: string;
      type: string;
      start: string;
      end: string;
      duration: string;
      movable: string;
      addSubmit: string;
      added: string;
      listTitle: string;
      empty: string;
      movableBadge: string;
      fixedBadge: string;
      minutes: string;
    };
    import: {
      title: string;
      subtitle: string;
      sourceRoot: string;
      fullScan: string;
      includePaths: string;
      includePathsHelp: string;
      previewSubmit: string;
      previewing: string;
      scanning: string;
      needPath: string;
      notComplete: string;
      commitSubmit: string;
      committing: string;
      needDecisions: string;
      runSubmit: string;
      importing: string;
      statusLine: string;
      statusStarting: string;
      statusRunning: string;
      staleHint: string;
    };
  };
  errors: {
    VALIDATION: string;
    NOT_FOUND: string;
    DATABASE: string;
    IO: string;
    UNKNOWN: string;
  };
  common: {
    taskCount_one: string;
    taskCount_other: string;
    loading: string;
    saving: string;
    creating: string;
    deadlineDate: string;
    actions: {
      activate: string;
      add: string;
      addExistingProject: string;
      addExistingTask: string;
      adding: string;
      apply: string;
      archive: string;
      back: string;
      cancel: string;
      close: string;
      collapse: string;
      complete: string;
      confirmArchive: string;
      continue: string;
      create: string;
      deactivate: string;
      defer: string;
      delete: string;
      deselect: string;
      dismiss: string;
      edit: string;
      expand: string;
      export: string;
      finish: string;
      next: string;
      pause: string;
      previous: string;
      remove: string;
      replan: string;
      restore: string;
      resume: string;
      save: string;
      saveChanges: string;
      skip: string;
      snooze: string;
      start: string;
      viewProgressDetails: string;
    };
    fields: {
      priority: string;
      status: string;
      targetDate: string;
      title: string;
    };
    labels: {
      taskStatus: {
        inbox: string;
        planned: string;
        in_progress: string;
        blocked: string;
        completed: string;
        cancelled: string;
        deferred: string;
      };
      priority: {
        none: string;
        low: string;
        normal: string;
        high: string;
        critical: string;
      };
      commitmentType: {
        sleep: string;
        meal: string;
        work: string;
        appointment: string;
        exercise: string;
        other: string;
      };
      goalStatus: {
        inbox: string;
        active: string;
        paused: string;
        achieved: string;
        abandoned: string;
        archived: string;
      };
      projectStatus: {
        inbox: string;
        planned: string;
        active: string;
        paused: string;
        completed: string;
        cancelled: string;
        archived: string;
      };
      effortSize: {
        trivial: string;
        small: string;
        medium: string;
        large: string;
        very_large: string;
      };
      deadlineStatus: {
        none: string;
        normal: string;
        approaching: string;
        overdue: string;
        blocked_risk: string;
      };
      progressState: {
        no_tasks: string;
        not_started: string;
        archived: string;
        cancelled: string;
        abandoned: string;
      };
      severity: {
        info: string;
        warning: string;
        error: string;
      };
      jobStatus: {
        queued: string;
        running: string;
        completed: string;
        failed: string;
        cancelled: string;
      };
      weekday: {
        sunday: string;
        monday: string;
        tuesday: string;
        wednesday: string;
        thursday: string;
        friday: string;
        saturday: string;
      };
      content: string;
      description: string;
      whyItMatters: string;
      taskTree: string;
    };
    markdown: {
      preview: string;
      previewRegion: string;
      empty: string;
      toolbarLabel: string;
      bold: string;
      italic: string;
      link: string;
      list: string;
      headingSelect: string;
      headingPlaceholder: string;
      headingOption: string;
      headingHelp: string;
      headingHelpTitle: string;
      viewModeLabel: string;
      viewSplit: string;
      viewEditor: string;
      viewPreview: string;
      dictionaryAdd: string;
      dictionaryAdded: string;
      dictionaryError: string;
    };
    export: {
      format: string;
      template: string;
      from: string;
      to: string;
      exportAll: string;
      exportButton: string;
      exporting: string;
      preparing: string;
      running: string;
      stalePreview: string;
      rePreview: string;
      valid: string;
      invalid: string;
      unavailable: string;
      formats: {
        markdown: string;
        typst: string;
        pdf: string;
      };
    };
    progress: {
      detailsFor: string;
      explainer: string;
      close: string;
      pointsSummary_one: string;
      pointsSummary_other: string;
      done: string;
      pointsValue: string;
      empty: string;
      loading: string;
    };
    exportPreview: {
      title: string;
      itemCount_one: string;
      itemCount_other: string;
      truncatedCount: string;
      largeSelection: string;
      therapyWarning: string;
      fingerprintLabel: string;
      fingerprintHelp: string;
      contentFingerprintTitle: string;
    };
    exportJob: {
      title: string;
      ariaLabel: string;
      failedSourceSaved: string;
      diagnostics: string;
      diagnosticCounts: string;
      copyDiagnostics: string;
      open: string;
      localArtifacts: string;
      unknownSize: string;
      bytes: string;
      kilobytes: string;
      megabytes: string;
      formats: {
        md: string;
        normalized_md: string;
        typ: string;
        pdf: string;
        diagnostics: string;
        unknown: string;
      };
    };
    exportHistory: {
      title: string;
      subtitle: string;
      count_one: string;
      count_other: string;
      empty: string;
      deleteConfirm: string;
      openFormat: string;
      saveCopy: string;
      noArtifact: string;
      formats: {
        md: string;
        typ: string;
        pdf: string;
      };
    };
    jobProgress: {
      title: string;
      ariaLabel: string;
      percent: string;
      percentMax: string;
      pending: string;
      cancelNote: string;
      backgroundNote: string;
      closeAria: string;
    };
    feedback: {
      errorTitle: string;
      reload: string;
    };
  };
  shell: {
    primaryNav: string;
    breadcrumb: string;
    nav: {
      today: string;
      analytics: string;
      calendar: string;
      projects: string;
      goals: string;
      tasks: string;
      journal: string;
      therapy: string;
      settings: string;
    };
    theme: {
      light: string;
      dark: string;
      system: string;
      switchToLight: string;
      switchToDark: string;
    };
    palette: {
      label: string;
      searchLabel: string;
      placeholder: string;
      note: string;
    };
    bell: {
      label: string;
      unread_one: string;
      unread_other: string;
      loading: string;
    };
  };
  today: {
    title: string;
    notToday: string;
    planningRitual: string;
    planStatus: string;
    planStatuses: {
      not_started: string;
      planning: string;
      planned: string;
      in_review: string;
      closed: string;
    };
    selectedTasks: string;
    blocks: {
      title: string;
      scheduled: {
        title: string;
        empty: string;
        loadMore: string;
        nextOccurrence: string;
        deleteConfirm: string;
        loadFailed: string;
      };
      inactive: {
        title: string;
        empty: string;
        loadFailed: string;
        oneTime: string;
        routine: string;
        reactivate: string;
        delete: string;
        deleteConfirm: string;
        reactivated: string;
        seriesReactivatedNotice: string;
      };
    };
    deadlineWarnings: string;
    fixedCommitments: string;
    noCommitments: string;
    task: {
      empty: string;
      moveUp: string;
      moveDown: string;
      divideIntoChildren: string;
    };
    replan: {
      move_to_next_day: string;
      return_to_planned: string;
      remove_date: string;
      defer: string;
      divide: string;
    };
    deadline: {
      empty: string;
      statuses: {
        overdue: string;
        blocked_risk: string;
        approaching: string;
        normal: string;
        none: string;
      };
    };
    block: {
      empty: string;
      add: string;
      noTime: string;
      recurring: string;
      deactivate: string;
      deactivateRoutine: string;
      delete: string;
      deleteRoutine: string;
      deleteConfirm: string;
      durationMinutes_one: string;
      durationMinutes_other: string;
      refreshFailed: string;
      types: {
        planning: string;
        focus: string;
        review: string;
        journaling: string;
        rest: string;
        personal: string;
      };
      statuses: {
        planned: string;
        active: string;
        completed: string;
        skipped: string;
        cancelled: string;
      };
      flexibilities: {
        fixed: string;
        flexible: string;
        approximate: string;
      };
      windows: {
        morning: string;
        afternoon: string;
        evening: string;
        after_meal: string;
      };
      form: {
        type: string;
        flexibility: string;
        label: string;
        labelPlaceholder: string;
        start: string;
        duration: string;
        badDuration: string;
        badStart: string;
        window: string;
        windowNone: string;
        note: string;
        submit: string;
        submitEdit: string;
        needLabel: string;
        weekly: string;
        repeatDays: string;
        needDays: string;
        needStartRecurring: string;
        recurrenceNotice: string;
        recurrenceLoadFailed: string;
        windowNotStored: string;
      };
      reactivate: {
        title: string;
        schedule: string;
        once: string;
        recurring: string;
        weekday: string;
        start: string;
        duration: string;
        submit: string;
      };
      task: {
        enable: string;
        pick: string;
        none: string;
        empty: string;
        loadFailed: string;
        workOnLabel: string;
      };
    };
    session: {
      title: string;
      start: string;
      saveFinish: string;
      outcomeAria: string;
      outcomePlaceholder: string;
      nextStepAria: string;
      nextStepPlaceholder: string;
      history: {
        empty: string;
        notStarted: string;
        until: string;
        next: string;
      };
      statuses: {
        planned: string;
        started: string;
        finished: string;
        paused: string;
        skipped: string;
      };
    };
    ritual: {
      title: string;
      steps: {
        unblocked: string;
        blocked: string;
        confirm: string;
        planning: string;
      };
      unblockedIntro: string;
      unblockedEmpty: string;
      blockedIntro: string;
      blockedEmpty: string;
      blockingNote: string;
      blockedByDependency: string;
      confirmIntro: string;
      selectionEmpty: string;
      reasonFor: string;
      reasons: {
        next_action: string;
        priority: string;
        deadline: string;
        dependency: string;
        manual: string;
      };
      capacityHint: string;
      capacityHintPreferred_one: string;
      capacityHintPreferred_other: string;
      capacityNotePrefill: string;
      capacityNotePrefillEnergy: string;
      intention: string;
      intentionPlaceholder: string;
      capacityNote: string;
      planningBlockLabel: string;
      finishHelp: string;
      finish: string;
    };
  };
  analytics: {
    title: string;
    subtitle: string;
    range: {
      from: string;
      to: string;
      invalid: string;
      showing: string;
    };
    empty: string;
    unnamedRoutine: string;
    outOfRangeLabelNote: string;
    coverage: {
      since: string;
      paused: string;
      unknown: string;
    };
    unrecordedNote: string;
    rate: {
      display: string;
      hidden: string;
    };
    definition: string;
    counts: {
      scheduled: string;
      completed: string;
      started: string;
      skipped: string;
      cancelled: string;
      unrecorded: string;
    };
    trend: {
      title: string;
      weekStarting: string;
    };
    reviewRoutine: string;
  };
  calendar: {
    title: string;
    weekView: string;
    monthView: string;
    legend: {
      dailyPlan: string;
      blocks: string;
      deadlines: string;
    };
    dots: {
      plan: string;
      blocks: string;
      deadline: string;
    };
  };
  goals: {
    title: string;
    new: string;
    subtitle: string;
    empty: string;
    detailTitle: string;
    editTitle: string;
    successDefinition: string;
    linkedProjects: string;
    linkedProjectsEmpty: string;
    directTasks: string;
    directTasksEmpty: string;
    addProject: string;
    addDirectTask: string;
    attachTaskDescription: string;
    attachProjectDescription: string;
    layoutGroupLabel: string;
    layoutList: string;
    layoutColumns: string;
    layoutTiles: string;
    form: {
      submit: string;
      createSubmit: string;
      description: string;
      why: string;
      success: string;
    };
  };
  projects: {
    title: string;
    new: string;
    subtitle: string;
    empty: string;
    detailTitle: string;
    editTitle: string;
    linkedToGoalNote: string;
    definitionOfDone: string;
    addTask: string;
    attachTaskDescription: string;
    taskTreeEmpty: string;
    searchLabel: string;
    searchPlaceholder: string;
    addExistingEmpty: string;
    layoutGroupLabel: string;
    layoutList: string;
    layoutColumns: string;
    layoutTiles: string;
    form: {
      submit: string;
      createSubmit: string;
      description: string;
      why: string;
      dod: string;
    };
  };
  tasks: {
    title: string;
    listTitle: string;
    new: string;
    empty: string;
    detailTitle: string;
    editTitle: string;
    create: {
      newTitle: string;
      newInProject: string;
      goalLinkNote: string;
      newChild: string;
      projectLoadingContext: string;
    };
    childrenArchived: string;
    reopen: string;
    changeStatus: string;
    changeStatusPlaceholder: string;
    blockingNote: string;
    blockingNotePlaceholder: string;
    archiveSubtree: string;
    restoreSubtree: string;
    archiveSubtreeHint: string;
    delete: string;
    confirmDelete: string;
    closureReason: string;
    prerequisites: string;
    createChild: string;
    addChild: string;
    archived: string;
    blockedByPrerequisite: string;
    searchLabel: string;
    searchPlaceholder: string;
    addExistingEmpty: string;
    pointsSuffix: string;
    effortSize: string;
    effortNotSet: string;
    scheduledDate: string;
    dependenciesLoading: string;
    addPrerequisite: string;
    selectPrerequisite: string;
    noPrerequisites: string;
    notBlocked: string;
    whyBlocked: string;
    layoutGroupLabel: string;
    layoutList: string;
    layoutColumns: string;
    layoutTiles: string;
    controlsLabel: string;
    sortLabel: string;
    sortCreated: string;
    sortProximity: string;
    sortImportance: string;
    sortDifficulty: string;
    sortAscending: string;
    sortDescending: string;
    filterLabel: string;
    filterAll: string;
    filterOpen: string;
    filterCompleted: string;
    filterOverdue: string;
    groupLabel: string;
    groupNone: string;
    groupProject: string;
    groupGoal: string;
    groupNested: string;
    ungrouped: string;
    noMatchingTasks: string;
    form: {
      submit: string;
      createSubmit: string;
      createChildSubmit: string;
      description: string;
      notes: string;
    };
    matrix: {
      title: string;
      corner: string;
      hint: string;
      recommendation: {
        whenSpace: string;
        plan: string;
        addressSoon: string;
        addressToday: string;
        pending: string;
        prioritize: string;
        immediatePriority: string;
        divideAndPrioritize: string;
        divideAndAddress: string;
        divide: string;
        urgentlyDivide: string;
      };
    };
  };
  journal: {
    title: string;
    new: string;
    empty: string;
    selectAll: string;
    deselectAll: string;
    setExportable: string;
    setPrivate: string;
    archiveSelected: string;
    exportable: string;
    private: string;
    selectedCount_one: string;
    selectedCount_other: string;
    listedCount_one: string;
    listedCount_other: string;
    selectEntryAria: string;
    archiveConfirm_one: string;
    archiveConfirm_other: string;
    archived_one: string;
    archived_other: string;
    markedExportable_one: string;
    markedExportable_other: string;
    markedPrivate_one: string;
    markedPrivate_other: string;
    newTitle: string;
    editTitle: string;
    detailTitle: string;
    linkedItems: string;
    olderEntry: string;
    newerEntry: string;
    form: {
      saveSubmit: string;
      createSubmit: string;
      date: string;
      content: string;
      mood: string;
      moodPlaceholder: string;
      energy: string;
      energyPlaceholder: string;
      emotions: string;
      emotionsPlaceholder: string;
      emotionsHelp: string;
      template: string;
      templateFallbackOption: string;
      exportableHelp: string;
    };
    templateField: {
      select: string;
    };
    reference: {
      loading: string;
      empty: string;
      entityTypeAria: string;
      entityIdAria: string;
      entityIdPlaceholder: string;
      relationAria: string;
      entityTypes: {
        task: string;
        project: string;
        goal: string;
        block: string;
        session: string;
      };
      relations: {
        mentions: string;
        related_to: string;
        reflects_on: string;
        result_of: string;
      };
    };
    export: {
      title: string;
      selected: string;
      empty: string;
      exportAllNote: string;
      defaultTitle: string;
    };
  };
  therapy: {
    title: string;
    privacyBanner: string;
    isolationNote: string;
    newCollection: string;
    collectionNamePlaceholder: string;
    collections: string;
    collectionsEmpty: string;
    layoutGroupLabel: string;
    layoutList: string;
    layoutColumns: string;
    layoutTiles: string;
    collection: string;
    collectionFallbackTitle: string;
    invalidCollectionId: string;
    addRecord: string;
    recordsEmpty: string;
    recordDetailTitle: string;
    newRecordTitle: string;
    editRecordTitle: string;
    createRecordSubmit: string;
    linkedTasks: string;
    recordTypes: {
      note: string;
      exercise: string;
      session_note: string;
      resource: string;
      reflection: string;
    };
    recordStatuses: {
      open: string;
      in_progress: string;
      completed: string;
      archived: string;
    };
    form: {
      type: string;
      occurredOn: string;
      content: string;
      saveSubmit: string;
    };
    reference: {
      taskId: string;
      taskIdPlaceholder: string;
      relation: string;
      note: string;
      add: string;
      empty: string;
      needTaskId: string;
      taskLoading: string;
      taskEmpty: string;
      taskLoadError: string;
      relations: {
        supports: string;
        prompted_by: string;
        evidence_for: string;
        related_to: string;
      };
    };
    export: {
      title: string;
      intro: string;
      selected: string;
      empty: string;
      exportAllNote: string;
      confirmPrivacy: string;
      noDate: string;
      defaultTitle: string;
    };
  };
  notifications: {
    title: string;
    subtitle: string;
    permission: {
      aria: string;
      title: string;
      checking: string;
      granted: string;
      notGranted: string;
      denied: string;
      unavailable: string;
      request: string;
    };
    privacyNote: string;
    rulesLoading: string;
    rulesEmpty: string;
    types: {
      planning: string;
      focus: string;
      journaling: string;
      review: string;
      deadline: string;
    };
    channels: {
      desktop: string;
      mobile: string;
      both: string;
    };
    escalations: {
      none: string;
      gentle: string;
    };
    deliveryStatuses: {
      scheduled: string;
      delivered: string;
      snoozed: string;
      dismissed: string;
      failed: string;
    };
    rule: {
      formAria: string;
      enableAria: string;
      enabled: string;
      sentToday_one: string;
      sentToday_other: string;
      sentNone: string;
      preferredTime: string;
      preferredTimeHelp: string;
      quietStart: string;
      quietEnd: string;
      quietEndHelp: string;
      deadlineWindow: string;
      deadlineWindowHelp: string;
      maxPerDay: string;
      snooze: string;
      snoozeHelp: string;
      safeMessage: string;
      safeMessageHelp: string;
      saveSubmit: string;
      saved: string;
      dailyLimitError: string;
      deadlineWindowError: string;
      quietHoursError: string;
      snoozeWholeError: string;
      snoozeRangeError: string;
      snoozeDuplicateError: string;
      snoozeCountError: string;
      safeMessageRequired: string;
      safeMessageTooLong: string;
    };
    history: {
      aria: string;
      title: string;
      privacyNote: string;
      loading: string;
      empty: string;
      rowLabel: string;
      rowLabelTarget: string;
      scheduled: string;
      delivered: string;
      snoozeAction: string;
      clear: string;
      clearAria: string;
      clearConfirm: string;
      cleared_one: string;
      cleared_other: string;
    };
    consent: {
      title: string;
      body: string;
      privacy: string;
      allow: string;
      requesting: string;
      notNow: string;
      later: string;
    };
  };
  import: {
    fingerprintLabel: string;
    entityTypes: {
      journal_entry: string;
      therapy_record: string;
      task: string;
      goal: string;
      project: string;
      note: string;
    };
    decisions: {
      ignore: string;
    };
    confidence: {
      high: string;
      medium: string;
      low: string;
      review: string;
    };
    report: {
      aria: string;
      title: string;
      source: string;
      status: string;
      summary: string;
      nothingImported: string;
      warnings: string;
      showWarnings: string;
      hideWarnings: string;
      noWarnings: string;
      reviewItems: string;
      reviewEmpty: string;
      untitled: string;
      decision: string;
      decisionSelect: string;
      decisionAria: string;
      accept: string;
      reject: string;
      placeholderNotice: string;
    };
  };
}

export const catalogs: Record<AppLocale, Translation> = {
  "en-US": enUS,
  "es-ES": esES,
  "de-DE": deDE,
  "fr-FR": frFR,
};

export const resources: Resource = {
  "en-US": { translation: enUS },
  "es-ES": { translation: esES },
  "de-DE": { translation: deDE },
  "fr-FR": { translation: frFR },
};

declare module "i18next" {
  interface CustomTypeOptions {
    defaultNS: "translation";
    resources: {
      translation: Translation;
    };
  }
}
