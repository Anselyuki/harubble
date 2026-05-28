<script lang="ts">
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import { Button } from '$lib/components/ui/button/index.js';

  interface Props {
    open: boolean;
    mode: 'create' | 'edit';
    initialName?: string;
    initialDescription?: string;
    onSubmit: (name: string, description: string) => void | Promise<void>;
    onClose: () => void;
  }

  let {
    open = $bindable(),
    mode,
    initialName = '',
    initialDescription = '',
    onSubmit,
    onClose,
  }: Props = $props();

  let name = $state('');
  let description = $state('');
  let submitting = $state(false);

  $effect(() => {
    if (open) {
      name = initialName;
      description = initialDescription;
      submitting = false;
    }
  });

  const isValid = $derived.by(() => name.trim().length > 0);
  const title = $derived.by(() =>
    mode === 'create' ? '新建合集' : '编辑合集'
  );
  const submitLabel = $derived.by(() => (mode === 'create' ? '创建' : '保存'));

  async function handleSubmit() {
    if (!isValid || submitting) return;
    submitting = true;
    try {
      await onSubmit(name.trim(), description.trim());
      onClose();
    } catch {
      // Error handled by caller via notifyError
    } finally {
      submitting = false;
    }
  }
</script>

<Dialog.Root
  bind:open
  onOpenChange={(v) => {
    if (!v) onClose();
  }}
>
  <Dialog.Content class="collection-form-dialog">
    <Dialog.Header>
      <Dialog.Title>{title}</Dialog.Title>
    </Dialog.Header>
    <div class="dialog-body">
      <label class="settings-field">
        <span>名称</span>
        <Input
          bind:value={name}
          placeholder="输入合集名称"
          class="h-9 border-[var(--dialog-border)] bg-[var(--dialog-control-bg)]"
        />
      </label>
      <label class="settings-field">
        <span>描述</span>
        <Input
          bind:value={description}
          placeholder="输入合集描述（可选）"
          class="h-9 border-[var(--dialog-border)] bg-[var(--dialog-control-bg)]"
        />
      </label>
    </div>
    <Dialog.Footer>
      <Button variant="outline" onclick={onClose} disabled={submitting}>
        取消
      </Button>
      <Button onclick={handleSubmit} disabled={!isValid || submitting}>
        {submitLabel}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
