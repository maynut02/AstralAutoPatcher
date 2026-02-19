namespace AstralAutoPatcher
{
  partial class Form1
  {
    private System.ComponentModel.IContainer components = null;

    protected override void Dispose(bool disposing)
    {
      if (disposing && (components != null))
      {
        components.Dispose();
      }
      base.Dispose(disposing);
    }

    private void InitializeComponent()
    {
      this.progressBar1 = new System.Windows.Forms.ProgressBar();
      this.lblStatus = new System.Windows.Forms.Label();
      this.lstLog = new System.Windows.Forms.ListBox();
      this.SuspendLayout();
      // 
      // progressBar1
      // 
      this.progressBar1.Location = new System.Drawing.Point(12, 40);
      this.progressBar1.Name = "progressBar1";
      this.progressBar1.Size = new System.Drawing.Size(552, 23);
      this.progressBar1.TabIndex = 0;
      // 
      // lblStatus
      // 
      this.lblStatus.AutoSize = true;
      this.lblStatus.Location = new System.Drawing.Point(12, 15);
      this.lblStatus.Name = "lblStatus";
      this.lblStatus.Size = new System.Drawing.Size(59, 15);
      this.lblStatus.TabIndex = 1;
      this.lblStatus.Text = "준비 중...";
      // 
      // lstLog
      // 
      this.lstLog.FormattingEnabled = true;
      this.lstLog.ItemHeight = 15;
      this.lstLog.Location = new System.Drawing.Point(12, 75);
      this.lstLog.Name = "lstLog";
      this.lstLog.Size = new System.Drawing.Size(552, 124);
      this.lstLog.TabIndex = 2;
      // 
      // Form1
      // 
      this.AutoScaleDimensions = new System.Drawing.SizeF(7F, 15F);
      this.AutoScaleMode = System.Windows.Forms.AutoScaleMode.Font;
      this.ClientSize = new System.Drawing.Size(576, 211);
      this.Controls.Add(this.lstLog);
      this.Controls.Add(this.lblStatus);
      this.Controls.Add(this.progressBar1);
      this.FormBorderStyle = System.Windows.Forms.FormBorderStyle.FixedDialog;
      this.MaximizeBox = false;
      this.Name = "Form1";
      this.StartPosition = System.Windows.Forms.FormStartPosition.CenterScreen;
      this.Text = "Astral Auto Patch";
      this.ResumeLayout(false);
      this.PerformLayout();
    }

    private System.Windows.Forms.ProgressBar progressBar1;
    private System.Windows.Forms.Label lblStatus;
    private System.Windows.Forms.ListBox lstLog;
  }
}
